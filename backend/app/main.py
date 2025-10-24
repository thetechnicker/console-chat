import asyncio
import hashlib
import os
from datetime import datetime, timedelta, timezone
from typing import Any, NamedTuple, Optional
import warnings
from uuid import uuid4
import jwt
import valkey.asyncio as valkey
from dotenv import load_dotenv
from fastapi import Body, Depends, FastAPI, HTTPException, Query, Security, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse, StreamingResponse
from fastapi.security import APIKeyHeader, HTTPAuthorizationCredentials, HTTPBearer
from jwt import PyJWTError
from sqlalchemy.orm import Session
from sqlalchemy import select

from pydantic import ValidationError

from app.datamodel import (
    BetterUser,
    PublicUser,
    ClientMessage,
    MessageType,
    ServerMessage,
    UserStatus,
)
from app.database import init_postgesql_connection, DBUser, DBPublicUser

# import psycopg2
# import psycopg2.extras
# from psycopg2 import pool

load_dotenv()

TTL = 60 * 60 * 24  # seconds
ALGORITHM = "HS256"
SECRET_KEY = os.getenv("SECRET", "secret")  # Secure random key recommended
if SECRET_KEY == "secret":
    warnings.warn("No secret given")

auth = HTTPBearer()  # Enforce auth
bearer_scheme = HTTPBearer(auto_error=False)  # Optional auth
api_key = APIKeyHeader(name="X-Api-Key")

# v = valkey.Valkey(host="valkey", port=6379, protocol=3)
v_pool = valkey.ConnectionPool(host="valkey", port=6379, protocol=3)
TOKEN_PREFIX = "session_token:"

postgreSQL_Session = init_postgesql_connection()
# postgreSQL_pool = pool.SimpleConnectionPool(
#    1,
#    20,
#    user=os.environ["POSTGRES_USER"],
#    password=os.environ["POSTGRES_PASSWORD"],
#    host="postgres",
#    port=5432,
#    database=os.getenv("POSTGRES_DB"),
# )

app = FastAPI()

origins = [
    "http://localhost",
    "http://localhost:8000",
    "http://127.0.0.1:8000",
]

app.add_middleware(
    CORSMiddleware,
    allow_origins=origins,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

Context = NamedTuple("Context", [("v", valkey.Valkey), ("p", Session)])


async def get_context():
    context = Context(
        valkey.Valkey.from_pool(v_pool),
        postgreSQL_Session(),
    )
    try:
        yield context
    finally:
        await context.v.aclose()
        context.p.close()  # or await if async session


def hash_password(password: str) -> str:
    return hashlib.sha256(
        password.encode()
    ).hexdigest()  # Simple hash, consider stronger methods


def create_access_token(
    data: dict[Any, Any], expires_delta: Optional[int] = None
) -> str:
    to_encode = data.copy()
    expire = datetime.now(timezone.utc) + timedelta(seconds=expires_delta or TTL)
    to_encode.update({"exp": expire, "iss": "http://localhost:8000/auth"})
    token = jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)  # type:ignore
    return token


async def get_current_user(
    credentials: Optional[HTTPAuthorizationCredentials] = Security(bearer_scheme),
    # context: Context = Depends(get_context),
) -> BetterUser:
    if credentials is None:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Missing credentials"
        )
    token = credentials.credentials
    try:
        payload: dict[Any, Any] = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
        print(payload)
        user = BetterUser.model_validate(payload)
        # stmt = select(DBUser).where(DBUser.username == user.username)
        # db_user = context.p.execute(stmt).scalar_one_or_none()

        # if db_user is None:
        #    raise HTTPException(
        #        status_code=status.HTTP_401_UNAUTHORIZED,
        #        detail="Invalid token: no user",
        #    )

        return user
    except (PyJWTError, ValidationError):
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid or expired token"
        )


def validate_api_key(key: str = Depends(api_key)):
    dest_key = os.environ.get("DEV_API_KEY")
    if dest_key and dest_key == key:
        return
    raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED)


@app.get("/")
async def root(_: None = Depends(validate_api_key)):
    return {"message": "Hello World"}


@app.post("/auth", response_model=UserStatus)
async def login(
    username: Optional[str] = Body(None),
    password: Optional[str] = Body(None),
    context: Context = Depends(get_context),
):
    password = hash_password(password) if password else password
    is_new = True
    if username and password:
        stmt = (
            select(DBUser)
            .where(DBUser.username == username)
            .where(DBUser.password_hash == password)
        )
        db_user = context.p.execute(stmt).scalars().one_or_none()
        if db_user:
            is_new = False
            user = BetterUser.model_validate(db_user)
        else:
            raise HTTPException(
                status_code=status.HTTP_401_UNAUTHORIZED, detail="Wrong Credentials"
            )
        # raise HTTPException(
        #    status_code=status.HTTP_501_NOT_IMPLEMENTED, detail=f"{db_user}"
        # )
    elif username:
        public = PublicUser(display_name=username)
        user = BetterUser(
            username=str(uuid4()),
            password_hash=None,
            private=True,
            public_data=public,
        )
    elif password:  # or username:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Incomplete login parameters",
        )
    else:
        public = PublicUser(display_name="anonymos")
        user = BetterUser(
            username=str(uuid4()),
            password_hash=None,
            private=True,
            public_data=public,
        )
    token = create_access_token(user.model_dump())
    return UserStatus(token=token, ttl=TTL, is_new=is_new)


@app.get("/valkey/status", response_class=JSONResponse)
async def get_valkey_status(
    _: None = Depends(validate_api_key),
    context: Context = Depends(get_context),
):
    try:
        # settings = v.get_connection_kwargs()  # type:ignore
        # print(settings)
        _ = await context.v.ping()
        # print(pong)
    except Exception:
        # Log the error e if desired
        raise HTTPException(status_code=503, detail="Service Unavailable")
    return {"status": "OK"}


@app.get("/users/status", response_model=BetterUser)
async def get_user_status(user: BetterUser = Depends(get_current_user)):
    return user


@app.post("/users/register")
async def register(
    password: str = Body(),
    user: BetterUser = Depends(get_current_user),
    overwrite_username: Optional[str] = Body(None),
    context: Context = Depends(get_context),
):
    try:
        # Check if already existing user
        stmt = select(DBUser).where(DBUser.username == user.username)
        db_user = context.p.execute(stmt).scalars().one_or_none()
        if db_user:
            raise HTTPException(
                status_code=status.HTTP_405_METHOD_NOT_ALLOWED,
                detail="You are already registered",
            )
        # Overwrite user name and check for availability
        user.username = overwrite_username or user.username
        stmt = select(DBUser).where(DBUser.username == user.username)
        if context.p.execute(stmt).scalars().one_or_none():
            raise HTTPException(
                status_code=status.HTTP_409_CONFLICT,
                detail=f"User {user.username} already exists",
            )

        # Hash password
        user.password_hash = hash_password(password)

        # Create User DB entry
        public_user = DBPublicUser(**user.public_data.model_dump())
        db_user = DBUser(**user.model_dump(db=True), public_data=public_user)
        context.p.add(db_user)
        context.p.commit()
        # raise Exception()
    except:
        raise HTTPException(status_code=status.HTTP_418_IM_A_TEAPOT)
    else:
        return {"status": "sugsesfully registered"}


@app.post("/room/{room}")
async def send(
    room: str,
    message: ClientMessage,  # Use the new Message model
    user: BetterUser = Depends(get_current_user),
    context: Context = Depends(get_context),
):
    msg = ServerMessage(
        user=user.public_data,
        text=message.text,
        # timestamp=datetime.now(timezone.utc),
        type=MessageType.TEXT,
    )
    await context.v.publish(
        room, msg.model_dump_json()
    )  # Use model_dump_json for serialization
    return {"message": f"send successful by user {user.public_data.display_name}"}


@app.get("/room/{room}", response_model=ServerMessage)
async def get(
    room: str,
    listen_seconds: int = Query(30, description="How long to listen in seconds"),
    user: BetterUser = Depends(get_current_user),
    context: Context = Depends(get_context),
):
    await context.v.publish(
        room,
        ServerMessage(
            type=MessageType.JOIN,
            text=f"User: {user.public_data.display_name} Joined",
            # timestamp=datetime.now(timezone.utc),
            user=user.public_data,  # Assign new user model
        ).model_dump_json(),
    )
    return StreamingResponse(
        get_message(room, timeout=listen_seconds, context=context),
        media_type="application/json",
    )


async def get_message(
    room: str,
    timeout: int,
    context: Context = Depends(get_context),
):
    async with context.v.pubsub() as pubsub:
        await pubsub.subscribe(room)

        end_time = asyncio.get_event_loop().time() + timeout

        while True:
            remaining = end_time - asyncio.get_event_loop().time()
            if remaining <= 0:
                yield b'{"event":"timeout"}'
                break
            try:
                message = await pubsub.get_message(
                    ignore_subscribe_messages=True, timeout=remaining
                )
            except Exception:
                continue
            if message is not None:
                data: str | bytes | Any = message["data"]
                if isinstance(data, bytes):
                    yield data
                elif isinstance(data, str):
                    yield data.encode()
                yield str(data).encode()  # Yield messages as bytes


# Uncomment for the exit functionality, if you decide to implement user exit handling
# @app.post("/api/exit/{room}")
# async def exit(room: str, user: DisplayUser = Depends(get_current_user)):
#     if user.username == "anonymous":
#         return {"message": "anonymous user exit does not affect subscriptions"}
#     user_channel = f"user_exit:{user.username}"
#     await v.publish(user_channel, STOPWORD)  # type:ignore
#     return {"message": f"exit successful for user {user.username}"}
