from __future__ import annotations

import hashlib
import json
import logging
import os
import uuid
from contextlib import asynccontextmanager
from datetime import datetime, timedelta, timezone
from typing import Annotated, Any, AsyncGenerator, NamedTuple, Optional

import jwt
import valkey.asyncio as valkey
from argon2 import PasswordHasher
from cryptography.fernet import Fernet
from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ec import EllipticCurvePrivateKey
from cryptography.hazmat.primitives.asymmetric.ed448 import Ed448PrivateKey
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives.asymmetric.rsa import RSAPrivateKey
from cryptography.hazmat.primitives.asymmetric.types import PrivateKeyTypes
from dotenv import load_dotenv
from fastapi import Depends, FastAPI, HTTPException, Security, status
from fastapi.security import APIKeyHeader, HTTPAuthorizationCredentials, HTTPBearer
from pydantic import BaseModel, Field
from sqlmodel import Session, select

from app.datamodel import init_postgesql_connection
from app.datamodel.user import PermanentUserPrivate, User, UserPrivate, UserType

# Allowed private key types for JWT signing
AllowedPrivateKeys = (
    RSAPrivateKey | EllipticCurvePrivateKey | Ed25519PrivateKey | Ed448PrivateKey
)

load_dotenv()
logger = logging.getLogger("dependencies")

# Environment and configuration
ENV = os.getenv("ENVIRONMENT", "development")
PRODUCTION = ENV.upper() == "PRODUCTION"
LEAVE_DELAY = 10  # seconds before being marked offline
TOKEN_PREFIX = "session_token:"
ISS = os.getenv("HOSTNAME", "https://localhost/")
TOKEN_TTL = 60 * 30  # default token expiration in seconds

# Error responses
RESPONSES: dict[int, dict[str, Any]] = {401: {"description": "Unauthorized"}}


def fallback_signing() -> tuple[str, bytes, bytes]:
    """Fallback to a default signing key if no secure key is provided."""
    algorithm = "HS256"
    jwt_key = os.getenv("SECRET", "secret").encode()

    if jwt_key == b"secret":
        logger.warning("No secret key provided! Please set a secure one.")
        if PRODUCTION:
            logger.warning("Generating signing key for JWT in production!")
            jwt_key = Fernet.generate_key()
    return algorithm, jwt_key, jwt_key


def is_valid_private_key(key: PrivateKeyTypes) -> bool:
    """Check if the provided key is valid for JWT signing."""
    return isinstance(key, AllowedPrivateKeys)


def setup_token_signing() -> tuple[str, AllowedPrivateKeys | str | bytes, bytes]:
    """Set up JWT token signing using secure keys."""
    password = os.getenv("PASSWORD", "password")

    if password == "password":
        logger.warning("No password provided! Please set a secure one.")
        return fallback_signing()

    algorithm = "RS256"
    try:
        # Load private key from PEM file
        with open("private_key.pem", "rb") as f:
            private_jwt_key = serialization.load_pem_private_key(
                f.read(), password=password.encode(), backend=default_backend()
            )
        if not is_valid_private_key(private_jwt_key):
            raise ValueError("Invalid private key type for JWT signing.")

        # for static typechecking
        assert isinstance(private_jwt_key, AllowedPrivateKeys)

        # Load public key from PEM file
        with open("public_key.pem", "rb") as f:
            public_jwt_key = f.read()
    except Exception as e:
        logger.error(f"Failed to load keys: {e}")
        return fallback_signing()

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
    """Validate the provided API key against the stored key."""
    dest_key = os.getenv("DEV_API_KEY")
    if dest_key and dest_key == key:
        return
    logger.warning("Invalid API Key provided.")
    raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED)


# Global variables
v_pool = None
engine = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Lifespan context manager to initialize resources."""
    global v_pool, engine
    valkey_host = os.getenv("VALKEY_HOST", "valkey")
    v_pool = valkey.ConnectionPool(host=valkey_host, port=6379, protocol=3)
    engine = init_postgesql_connection()

    yield  # Yield control to the application

    logger.info("Closing database connections...")
    await v_pool.aclose()


async def get_db_context() -> AsyncGenerator[DatabaseContext, Any]:
    """Retrieve the database context for dependency injection."""
    if v_pool is None or engine is None:
        logger.error("Database connections weren't initialized correctly.")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Database connections weren't initialized correctly.",
        )
    with Session(engine) as session:
        valkey_instance = valkey.Valkey(connection_pool=v_pool, decode_responses=True)
        yield DatabaseContext(valkey=valkey_instance, psql_session=session)
        await valkey_instance.aclose()


DatabaseDependency = Annotated[DatabaseContext, Depends(get_db_context)]
TokenDependency = Annotated[HTTPAuthorizationCredentials, Security(auth_bearer_scheme)]
OptionalTokenDependency = Annotated[
    Optional[HTTPAuthorizationCredentials], Security(optional_auth_bearer_scheme)
]
ApiKeyAuth = Annotated[None, Security(validate_api_key)]


async def get_user_from_token(
    token: str, db: DatabaseDependency
) -> UserPrivate | PermanentUserPrivate:
    """Extract the user from provided JWT token."""
    try:
        payload = jwt.decode(token, PUBLIC_JWT_KEY, algorithms=[ALGORITHM])
        user_id = payload.get("sub")
        if not payload.get("tmp", True):
            stmt = select(User).where(User.id == user_id)
            user = db.psql_session.exec(stmt).one_or_none()
            return PermanentUserPrivate.model_validate(user)
        else:
            res = await db.valkey.get(user_id)
            logger.debug(f"Temporary user data retrieved: {res}")
            return UserPrivate.model_validate_json(res)
    except jwt.ExpiredSignatureError:
        logger.warning("Token has expired.")
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Token has expired."
        )
    except jwt.PyJWTError:
        logger.warning("Invalid token provided.")
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid token."
        )


async def get_current_user(
    credentials: TokenDependency, db: DatabaseDependency
) -> UserPrivate:
    """Get the currently authenticated user."""
    return await get_user_from_token(credentials.credentials, db)


async def get_current_permanent_user(
    credentials: TokenDependency, db: DatabaseDependency
) -> PermanentUserPrivate:
    """Get the currently authenticated permanent user."""
    user = await get_user_from_token(credentials.credentials, db)
    if user.user_type == UserType.GUEST or not isinstance(user, PermanentUserPrivate):
        logger.warning("Guest user attempted to access restricted resource.")
        raise HTTPException(
            status.HTTP_401_UNAUTHORIZED,
            detail="Only registered users can create a room.",
        )
    return user


UserDependency = Annotated[UserPrivate, Depends(get_current_user)]
PermanentUserDependency = Annotated[
    PermanentUserPrivate, Depends(get_current_permanent_user)
]


class UUIDEncoder(json.JSONEncoder):
    def default(self, o: Any) -> Any:
        """Override default to serialize UUIDs."""
        if isinstance(o, uuid.UUID):
            return str(o)  # Convert UUID to string
        return super().default(o)


ph = PasswordHasher()


def secure_hash_argon2(username: str, password: str) -> str:
    """Securely hash a password using Argon2."""
    combined = username + password  # Combine username and password
    return ph.hash(combined)  # Create the hash


def verify_password(hashed: str, username: str, password: str) -> bool:
    """Verify the password against the hashed password."""
    combined = username + password
    try:
        return ph.verify(
            hashed, combined
        )  # Will raise an exception if the hash does not match
    except Exception as e:
        logger.error(f"Password verification failed: {e}")
        return False


def get_from_login(
    username: str, password: str, db: DatabaseDependency
) -> Optional[UserPrivate]:
    """Retrieve user data for login validation."""
    stmt = select(User).where(User.username == username)
    user = db.psql_session.exec(stmt).one_or_none()
    if (
        user
        and user.password
        and verify_password(hashed=user.password, username=username, password=password)
    ):
        return UserPrivate.model_validate(user)
    logger.warning(f"Login attempt failed for user: {username}")
    return None


def create_access_token(
    user: User | UserPrivate,
    expires_delta: int = TOKEN_TTL,
    iss_sufix: str = "",
) -> Token:
    """Create a JWT access token for the given user."""
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
    logger.info(f"Access token created for user: {user.username}")
    return Token(token=token_str, ttl=expires_delta)


def deterministic_color_from_string(input_string: str) -> str:
    """Generate a consistent color hex code from an input string."""
    # Hash the input string using SHA-256 to get a consistent fixed-length hash
    hash_bytes = hashlib.sha256(input_string.encode("utf-8")).hexdigest()
    color = hash_bytes[0:6]  # Take first six characters for hex color code
    logger.debug(f"Deterministic color generated for '{input_string}': #{color}")
    return f"#{color}"
