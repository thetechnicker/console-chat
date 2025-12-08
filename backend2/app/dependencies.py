import hashlib
import json
import logging
import os
import uuid
import warnings
from contextlib import asynccontextmanager
from datetime import datetime, timedelta, timezone
from typing import Annotated, Any, NamedTuple, Optional

import jwt
import valkey.asyncio as valkey
from argon2 import PasswordHasher
from dotenv import load_dotenv
from fastapi import Depends, FastAPI, HTTPException, Security, status
from fastapi.security import APIKeyHeader  # OAuth2PasswordBearer,
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from pydantic import BaseModel, Field
from sqlmodel import Session, select

from app.datamodel import init_postgesql_connection
from app.datamodel.user import AppearancePublic, User, UserPrivate, UserType

load_dotenv()

LEAVE_DELAY = 10  # seconds before being marked offline
TOKEN_TTL = 60 * 60 * 24  # Token Time-to-Live in seconds
TOKEN_PREFIX = "session_token:"
ALGORITHM = "HS256"
SECRET_KEY = os.getenv("SECRET", "secret")  # Use a secure random key

if SECRET_KEY == "secret":
    warnings.warn("No secret key provided! Please set a secure one.")

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
# oauth2_scheme = OAuth2PasswordBearer(tokenUrl="users/token")


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
        payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
        id = payload.get("sub")
        if not payload.get("tmp", True):
            stmt = select(User).where(User.id == id)
            user = db.psql_session.exec(stmt).one_or_none()
            return UserPrivate.model_validate(user)
        else:
            res = await db.valkey.get(id)
            logging.debug(f"Temporary user data: {res}")
            return UserPrivate(
                appearance=AppearancePublic(color="#ffccff"), id=uuid.uuid4()
            )
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


# async def get_current_user_oauth(
#    token: Annotated[str, Depends(oauth2_scheme)], db: DatabaseDependency
# ) -> UserPrivate:
#    user = await get_user_from_token(token, db)
#    if not user:
#        raise HTTPException(
#            status_code=status.HTTP_401_UNAUTHORIZED,
#            detail="Not authenticated.",
#            headers={"WWW-Authenticate": "Bearer"},
#        )
#    return user


UserDependency = Annotated[UserPrivate, Depends(get_current_user)]


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


def create_access_token(
    user: User | UserPrivate, expires_delta: int = TOKEN_TTL
) -> Token:
    expire = datetime.now(timezone.utc) + timedelta(seconds=expires_delta or TOKEN_TTL)
    to_encode = {
        "exp": expire,
        "iat": datetime.now(timezone.utc),
        "iss": "http://localhost:8000/auth",
        "sub": str(user.id),
        "name": user.username,
        "tmp": user.user_type == UserType.GUEST,
    }
    token_str = jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)
    return Token(token=token_str, ttl=expires_delta)


def deterministic_color_from_string(input_string: str) -> str:
    # Hash the input string using SHA-256 to get a consistent fixed-length hash
    hash_bytes = hashlib.sha256(input_string.encode("utf-8")).hexdigest()
    color = hash_bytes[0:6]  # Take first six characters for hex color code
    return f"#{color}"
