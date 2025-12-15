import hashlib
import json
import logging
import os
import uuid
from contextlib import asynccontextmanager
from datetime import datetime, timedelta, timezone
from typing import Annotated, Any, NamedTuple, Optional

import jwt
import valkey.asyncio as valkey
from argon2 import PasswordHasher
from cryptography.fernet import Fernet
from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.types import PrivateKeyTypes
from dotenv import load_dotenv
from fastapi import Depends, FastAPI, HTTPException, Security, status
from fastapi.security import APIKeyHeader, HTTPAuthorizationCredentials, HTTPBearer
from jwt.algorithms import AllowedPrivateKeys
from pydantic import BaseModel, Field
from sqlmodel import Session, select

from app.datamodel import init_postgesql_connection
from app.datamodel.user import User, UserPrivate, UserType

load_dotenv()
logger = logging.getLogger("dependencies")

ENV = os.getenv("ENVIRONMENT", "development")
PRODUCTION = ENV.upper() == "PRODUCTION"

LEAVE_DELAY = 10  # seconds before being marked offline
TOKEN_PREFIX = "session_token:"
ISS = os.getenv("HOSTNAME", "https://localhost/")

use_fallback = True
TOKEN_TTL = 60 * 30


def fallback_signing() -> tuple[str, bytes]:
    algorithm = "HS256"
    jwt_key = os.getenv("SECRET", "secret").encode()
    if jwt_key == b"secret":
        logging.warning("No secret key provided! Please set a secure one.")
        if PRODUCTION:
            logging.warning("Generating signing key for JWT in production!")
            jwt_key = Fernet.generate_key()
    return algorithm, jwt_key


def is_valid_private_key(key: PrivateKeyTypes):
    """Check if the key is of an accepted type for JWT signing."""
    return isinstance(key, AllowedPrivateKeys)


def setup_token_signing() -> tuple[str, AllowedPrivateKeys | str | bytes, bytes]:
    password = os.getenv("PASSWORD", "password")
    if password == "password":
        logging.warning("No password provided! Please set a secure one.")
        algorithm, jwt_key = fallback_signing()
        return algorithm, jwt_key, jwt_key

    algorithm = "RS256"
    try:
        with open("private_key.pem", "rb") as f:
            private_jwt_key = serialization.load_pem_private_key(
                f.read(), password=password.encode(), backend=default_backend()
            )
        if not is_valid_private_key(private_jwt_key):
            raise ValueError(
                "Invalid private key type for JWT signing. Acceptable types are RSAPrivateKey, EllipticCurvePrivateKey, Ed25519PrivateKey, Ed448PrivateKey."
            )
        assert isinstance(private_jwt_key, AllowedPrivateKeys)

        with open("public_key.pem", "rb") as f:
            public_jwt_key = f.read()
    except Exception as e:
        logging.error(f"Failed to load keys: {e}")
        algorithm, jwt_key = fallback_signing()
        return algorithm, jwt_key, jwt_key

    return algorithm, private_jwt_key, public_jwt_key


ALGORITHM, PRIVATE_JWT_KEY, PUBLIC_JWT_KEY = setup_token_signing()


DatabaseContext = NamedTuple(
    "Context", [("valkey", valkey.Valkey), ("psql_session", Session)]
)


class Token(BaseModel):
    token: str
    ttl: int


class OnlineResponse(BaseModel):
    token: Token
    user: uuid.UUID


class LoginData(BaseModel):
    username: str
    password: str


class RegisterData(BaseModel):
    username: Optional[str] = Field(None)
    password: str


# Authentication schemes
auth_bearer_scheme = HTTPBearer()
optional_auth_bearer_scheme = HTTPBearer(auto_error=False)
api_key_scheme = APIKeyHeader(name="X-Api-Key")


def validate_api_key(key: Annotated[str, Security(api_key_scheme)]):
    dest_key = os.getenv("DEV_API_KEY")
    if dest_key and dest_key == key:
        return
    raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED)


# Global variables
v_pool = None
engine = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    global v_pool, engine
    valkey_host = os.getenv("VALKEY_HOST", "valkey")
    v_pool = valkey.ConnectionPool(host=valkey_host, port=6379, protocol=3)
    engine = init_postgesql_connection()

    # NOTE: might be usefull if a server reboot should invalidate every temp user
    # v = valkey.Valkey.from_pool(v_pool)
    # keys = await v.keys()
    # for key in keys:
    #    await v.delete(key)

    yield


def get_db_context():
    if v_pool is None or engine is None:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Database connections weren't initialized correctly.",
        )
    with Session(engine) as session:
        yield DatabaseContext(
            valkey=valkey.Valkey.from_pool(v_pool), psql_session=session
        )


DatabaseDependency = Annotated[DatabaseContext, Depends(get_db_context)]
TokenDependency = Annotated[HTTPAuthorizationCredentials, Security(auth_bearer_scheme)]
OptionalTokenDependency = Annotated[
    Optional[HTTPAuthorizationCredentials], Security(optional_auth_bearer_scheme)
]
ApiKeyAuth = Annotated[None, Security(validate_api_key)]


async def get_user_from_token(token: str, db: DatabaseDependency) -> UserPrivate:
    try:
        payload = jwt.decode(token, PUBLIC_JWT_KEY, algorithms=[ALGORITHM])
        id = payload.get("sub")
        if not payload.get("tmp", True):
            stmt = select(User).where(User.id == id)
            user = db.psql_session.exec(stmt).one_or_none()
            return UserPrivate.model_validate(user)
        else:
            res = await db.valkey.get(id)
            logging.debug(f"Temporary user data: {res}")
            return UserPrivate.model_validate_json(res)
    except jwt.ExpiredSignatureError:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Token has expired."
        )
    except jwt.PyJWTError:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid token."
        )


async def get_current_user(
    credentials: TokenDependency, db: DatabaseDependency
) -> UserPrivate:
    return await get_user_from_token(credentials.credentials, db)


async def get_current_permanent_user(
    credentials: TokenDependency, db: DatabaseDependency
) -> UserPrivate:
    user = await get_user_from_token(credentials.credentials, db)
    if user.user_type == UserType.GUEST:
        raise HTTPException(
            status.HTTP_401_UNAUTHORIZED,
            detail="only registered user can create a room",
        )
    return user


UserDependency = Annotated[UserPrivate, Depends(get_current_user)]
PermanentUserDependency = Annotated[UserPrivate, Depends(get_current_permanent_user)]


class UUIDEncoder(json.JSONEncoder):
    def default(self, o: Any) -> Any:
        if isinstance(o, uuid.UUID):
            return str(o)  # Convert UUID to string
        return super().default(o)


ph = PasswordHasher()


def secure_hash_argon2(username: str, password: str) -> str:
    combined = username + password  # Combine username and password
    return ph.hash(combined)  # Create the hash


def verify_password(hashed: str, username: str, password: str) -> bool:
    combined = username + password
    try:
        return ph.verify(
            hashed, combined
        )  # Will raise an exception if the hash does not match
    except Exception:
        return False


def get_from_login(username: str, password: str, db: DatabaseDependency):
    stmt = select(User).where(User.username == username)
    user = db.psql_session.exec(stmt).one_or_none()
    if (
        user
        and user.password
        and verify_password(hashed=user.password, username=username, password=password)
    ):
        return UserPrivate.model_validate(user)


def create_access_token(
    user: User | UserPrivate,
    expires_delta: int = TOKEN_TTL,
    iss_sufix: str = "",
) -> Token:
    expire = datetime.now(timezone.utc) + timedelta(seconds=expires_delta or TOKEN_TTL)
    to_encode = {
        "exp": expire,
        "iat": datetime.now(timezone.utc),
        "iss": ISS + iss_sufix,
        "sub": str(user.id),
        "name": user.username,
        "tmp": user.user_type == UserType.GUEST,
    }
    token_str = jwt.encode(to_encode, PRIVATE_JWT_KEY, algorithm=ALGORITHM)
    return Token(token=token_str, ttl=expires_delta)


def deterministic_color_from_string(input_string: str) -> str:
    # Hash the input string using SHA-256 to get a consistent fixed-length hash
    hash_bytes = hashlib.sha256(input_string.encode("utf-8")).hexdigest()
    color = hash_bytes[0:6]  # Take first six characters for hex color code
    return f"#{color}"
