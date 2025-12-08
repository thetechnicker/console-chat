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
from fastapi.exceptions import HTTPException
from fastapi.security import (
    APIKeyHeader,
    HTTPAuthorizationCredentials,
    HTTPBearer,
    OAuth2PasswordBearer,
)
from pydantic import BaseModel, Field
from sqlmodel import Session, select

from app.datamodel import init_postgesql_connection
from app.datamodel.user import AppearancePublic, User, UserPrivate, UserType

load_dotenv()

LEAVE_DELAY = 10  # How long between requests to `/room/{room_name}` before being marked as offline

TOKEN_TTL = 60 * 60 * 24  # seconds
TOKEN_PREFIX = "session_token:"

ALGORITHM = "HS256"
SECRET_KEY = os.getenv("SECRET", "secret")  # Secure random key recommended
if SECRET_KEY == "secret":
    warnings.warn("No secret given")

DatabaseContext = NamedTuple(
    "Context", [("valkey", valkey.Valkey), ("psql_session", Session)]
)


class Token(BaseModel):
    token: str
    ttl: int
    # is_new: bool


class OnlineResponce(BaseModel):
    token: Token
    user: uuid.UUID


class LoginData(BaseModel):
    username: str
    password: str


class RegisterData(BaseModel):
    username: Optional[str] = Field(None)
    password: str


auth_bearer_scheme = HTTPBearer()  # Enforce auth
optional_auth_bearer_scheme = HTTPBearer(auto_error=False)  # Optional auth
api_key_scheme = APIKeyHeader(name="X-Api-Key")
oauth2_scheme = OAuth2PasswordBearer(tokenUrl="users/token")


def validate_api_key(key: Annotated[str, Security(api_key_scheme)]):
    dest_key = os.environ.get("DEV_API_KEY")
    if dest_key and dest_key == key:
        return
    raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED)


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
            detail="The database connections whererent initialized correctly",
        )
    with Session(engine) as session:
        yield DatabaseContext(
            valkey=valkey.Valkey.from_pool(v_pool), psql_session=session
        )


DatabaseDependencie = Annotated[DatabaseContext, Depends(get_db_context)]
TokenDependencie = Annotated[HTTPAuthorizationCredentials, Security(auth_bearer_scheme)]
OptionalTokenDependencie = Annotated[
    Optional[HTTPAuthorizationCredentials], Security(optional_auth_bearer_scheme)
]
ApiKeyAuth = Annotated[None, Security(validate_api_key)]


async def get_user_from_token(token: str, db: DatabaseDependencie) -> UserPrivate:
    try:
        # Decode the JWT token
        payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
        id = payload.get("id")
        if not payload.get("tmp", True):
            stmt = select(User).where(User.id == id)
            res = db.psql_session.exec(stmt)
            user = res.one_or_none()
        else:
            res = await db.valkey.get(id)
            logging.root.debug(f"{res}")
            # user = User.model_validate_json(res)
            return UserPrivate(
                appearance=AppearancePublic(color="#ffccff"), id=uuid.uuid4()
            )
        return UserPrivate.model_validate(user)
    except jwt.ExpiredSignatureError:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Token has expired"
        )
    except jwt.PyJWTError:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid token"
        )


async def get_current_user(
    credentials: TokenDependencie,
    db: DatabaseDependencie,
) -> UserPrivate:
    return await get_user_from_token(credentials.credentials, db)


async def get_current_user_oauth(
    token: Annotated[str, Depends(oauth2_scheme)], db: DatabaseDependencie
) -> UserPrivate:
    user = await get_user_from_token(token, db)
    if not user:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Not authenticated",
            headers={"WWW-Authenticate": "Bearer"},
        )
    return user


UserDependencie = Annotated[UserPrivate, Depends(get_current_user_oauth)]


class UUIDEncoder(json.JSONEncoder):
    def default(self, o: Any):
        if isinstance(o, uuid.UUID):
            return str(o)  # Convert UUID to string
        return super().default(o)


ph = PasswordHasher()


def secure_hash_argon2(username: str, password: str):
    # Combine username and password
    combined = username + password
    # Create the hash
    hash_pw = ph.hash(combined)
    return hash_pw


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
        "iss": "http://localhost:8000/auth",
        "id": str(user.id),
        "tmp": True if user.user_type == UserType.GUEST else False,
    }
    token_str = jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)
    token = Token(token=token_str, ttl=expires_delta)
    return token


def deterministic_color_from_string(input_string: str) -> str:
    # Hash the input string using SHA-256 to get a consistent fixed-length hash
    hash_bytes = hashlib.sha256(input_string.encode("utf-8")).hexdigest()
    # Convert first three bytes of hash to integers for RGB
    color = hash_bytes[0:6]
    return f"#{color}"
