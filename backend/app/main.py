import asyncio
import hashlib
import os
from datetime import datetime, timedelta, timezone
from typing import Any, NamedTuple, Optional, cast
from uuid import uuid4

import jwt
import psycopg2
import valkey.asyncio as valkey
from dotenv import load_dotenv
from fastapi import Body, Depends, FastAPI, HTTPException, Query, Security, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse, StreamingResponse
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from jwt import PyJWTError
from psycopg2 import pool
from pydantic import ValidationError

from app.datamodel import (
    ClientMessage,
    MessageType,
    ServerMessage,
    User,
    UserConfig,
    UserStatus,
)

# import psycopg2.extras

load_dotenv()

TTL = 3600  # seconds
ALGORITHM = "HS256"
SECRET_KEY = os.getenv("SECRET")  # Secure random key recommended

auth = HTTPBearer()  # Enforce auth
bearer_scheme = HTTPBearer(auto_error=False)  # Optional auth

# v = valkey.Valkey(host="valkey", port=6379, protocol=3)
v_pool = valkey.ConnectionPool(host="valkey", port=6379, protocol=3)
TOKEN_PREFIX = "session_token:"

postgreSQL_pool = pool.SimpleConnectionPool(
    1,
    20,
    user=os.environ["POSTGRES_USER"],
    password=os.environ["POSTGRES_PASSWORD"],
    host="postgres",
    port=5432,
    database="DEBUG",
)

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

Context = NamedTuple(
    "Context", [("v", valkey.Valkey), ("p", psycopg2.extensions.connection)]
)


def get_context():
    return Context(
        valkey.Valkey.from_pool(v_pool),
        cast(psycopg2.extensions.connection, postgreSQL_pool.getconn()),
    )


def hash_password(password: str) -> str:
    return hashlib.sha256(
        password.encode()
    ).hexdigest()  # Simple hash, consider stronger methods


def create_access_token(
    data: dict[Any, Any], expires_delta: Optional[int] = None
) -> str:
    to_encode = data.copy()
    expire = datetime.now(timezone.utc) + timedelta(seconds=expires_delta or TTL)
    to_encode.update({"exp": expire})
    token = jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)  # type:ignore
    return token


async def get_current_user(
    credentials: Optional[HTTPAuthorizationCredentials] = Security(bearer_scheme),
) -> UserConfig:
    if credentials is None:
        return UserConfig(display_name="anonymous")
    token = credentials.credentials
    try:
        payload: dict[Any, Any] = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
        # print(payload)
        user = User.model_validate(payload)
        # TODO: User validation

        #    raise HTTPException(
        #        status_code=status.HTTP_401_UNAUTHORIZED,
        #        detail="Invalid token: no user",
        #    )

        return user.config
    except (PyJWTError, ValidationError):
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid or expired token"
        )


@app.get("/")
async def root():
    return {"message": "Hello World"}


@app.post("/login", response_model=UserStatus)
async def login(
    username: Optional[str] = Body(None),
    password: Optional[str] = Body(None),
):
    if username:
        user = User(
            username=username,
            password_hash=hash_password(password) if password else "",
            private=True,
            config=UserConfig(display_name=username),
        )
    else:
        user = User(
            username=str(uuid4()),
            password_hash="",
            private=True,
            config=UserConfig(display_name="anonym"),
        )
    token = create_access_token(user.model_dump())
    return UserStatus(token=token, ttl=TTL, is_new=True)


@app.get("/valkey/status", response_class=JSONResponse)
async def get_valkey_status(context: Context = Depends(get_context)):
    try:
        # settings = v.get_connection_kwargs()  # type:ignore
        # print(settings)
        _ = await context.v.ping()
        # print(pong)
    except Exception:
        # Log the error e if desired
        raise HTTPException(status_code=503, detail="Service Unavailable")
    return {"status": "OK"}


@app.get("/user/status", response_model=UserConfig)
async def get_user_status(user: UserConfig = Depends(get_current_user)):
    return user


@app.get("/room/{room}")
async def get(
    room: str,
    listen_seconds: int = Query(30, description="How long to listen in seconds"),
    user: UserConfig = Depends(get_current_user),
    context: Context = Depends(get_context),
):
    v = context.v
    await v.publish(
        room,
        ServerMessage(
            type=MessageType.JOIN,
            text=f"User: {user.display_name} Joined",
            # timestamp=datetime.now(timezone.utc),
            user=user,  # Assign new user model
        ).model_dump_json(),
    )
    return StreamingResponse(
        get_message(room, timeout=listen_seconds),
        media_type="application/json",
    )


@app.post("/room/{room}")
async def send(
    room: str,
    message: ClientMessage,  # Use the new Message model
    user: UserConfig = Depends(get_current_user),
    context: Context = Depends(get_context),
):
    v = context.v
    msg = ServerMessage(
        user=user,
        text=message.text,
        # timestamp=datetime.now(timezone.utc),
        type=MessageType.TEXT,
    )
    await v.publish(
        room, msg.model_dump_json()
    )  # Use model_dump_json for serialization
    return {"message": f"send successful by user {user.display_name}"}


async def get_message(
    room: str,
    timeout: int,
    context: Context = Depends(get_context),
):
    v = context.v
    async with v.pubsub() as pubsub:
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
