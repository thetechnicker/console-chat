import hashlib
import os
import warnings
from contextlib import asynccontextmanager
from datetime import datetime, timedelta, timezone
from typing import Annotated, Any, NamedTuple, Optional

import jwt
import valkey.asyncio as valkey
from dotenv import load_dotenv
from fastapi import Depends, FastAPI, HTTPException, status
from fastapi.security import APIKeyHeader, HTTPBearer
from sqlmodel import Session, select

from app.database import Message, init_postgesql_connection

load_dotenv()

LEAVE_DELAY = 10  # How long between requests to `/room/{room_name}` before being marked as offline

TOKEN_TTL = 60 * 60 * 24  # seconds
TOKEN_PREFIX = "session_token:"

ALGORITHM = "HS256"
SECRET_KEY = os.getenv("SECRET", "secret")  # Secure random key recommended
if SECRET_KEY == "secret":
    warnings.warn("No secret given")

auth = HTTPBearer()  # Enforce auth
bearer_scheme = HTTPBearer(auto_error=False)  # Optional auth
api_key = APIKeyHeader(name="X-Api-Key")


def deterministic_color_from_string(input_string: str) -> str:
    # Hash the input string using SHA-256 to get a consistent fixed-length hash
    hash_bytes = hashlib.sha256(input_string.encode("utf-8")).hexdigest()
    # Convert first three bytes of hash to integers for RGB
    color = hash_bytes[0:6]
    return f"#{color}"


v_pool = None
engine = None

DatabaseContext = NamedTuple(
    "Context", [("valkey", valkey.Valkey), ("psql_session", Session)]
)


@asynccontextmanager
async def lifespan(app: FastAPI):
    global v_pool, engine
    v_pool = valkey.ConnectionPool(host="valkey", port=6379, protocol=3)
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


SessionDep = Annotated[DatabaseContext, Depends(get_db_context)]


app = FastAPI(lifespan=lifespan)


def hash_password(password: str) -> str:
    return hashlib.sha256(
        password.encode()
    ).hexdigest()  # Simple hash, consider stronger methods


def create_access_token(
    data: dict[Any, Any], expires_delta: Optional[int] = None
) -> str:
    to_encode = data.copy()
    expire = datetime.now(timezone.utc) + timedelta(seconds=expires_delta or TOKEN_TTL)
    to_encode.update({"exp": expire, "iss": "http://localhost:8000/auth"})
    token = jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)  # type:ignore
    return token


def validate_api_key(key: str = Depends(api_key)):
    dest_key = os.environ.get("DEV_API_KEY")
    if dest_key and dest_key == key:
        return
    raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED)


API_KEY_AUTH = Annotated[None, Depends(validate_api_key)]


@app.get("/message")
def send(db_session: SessionDep):
    messages = db_session.psql_session.exec(select(Message)).all()
    return messages
