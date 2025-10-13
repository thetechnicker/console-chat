import asyncio
import hashlib
import os
from datetime import datetime, timedelta, timezone
from typing import Any, Optional
from uuid import uuid4

import jwt
import valkey.asyncio as valkey
from dotenv import load_dotenv
from fastapi import Body, Depends, FastAPI, HTTPException, Query, Security, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import StreamingResponse
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from jwt import PyJWTError
from pydantic import BaseModel

load_dotenv()

TTL = 3600  # seconds
ALGORITHM = "HS256"
SECRET_KEY = os.getenv("SECRET")  # Secure random key recommended

auth = HTTPBearer()  # Enforce auth
bearer_scheme = HTTPBearer(auto_error=False)  # Optional auth

v = valkey.Valkey(host="localhost", port=6379, protocol=3)
TOKEN_PREFIX = "session_token:"

STOPWORD = "STOP"

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


# User and message models
class DisplayUser(BaseModel):
    username: str
    display_name: str


class ClientMessage(BaseModel):
    message: str
    timestamp: datetime


class Message(ClientMessage):
    user: DisplayUser


class UserStatus(BaseModel):
    token: str
    ttl: int
    is_new: bool


def hash_password(password: str) -> str:
    # TODO: Replace with stronger hash (bcrypt or passlib recommended)
    return hashlib.sha256(password.encode()).hexdigest()


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
) -> DisplayUser:
    if credentials is None:
        # No token means anonymous user
        return DisplayUser(username="anonymous", display_name="anonymous")
    token = credentials.credentials
    try:
        payload: dict[Any, Any] = jwt.decode(  # type: ignore
            token, SECRET_KEY, algorithms=[ALGORITHM]
        )  # type:ignore
        username = payload.get("username")
        if username is None:
            raise HTTPException(
                status_code=status.HTTP_401_UNAUTHORIZED,
                detail="Invalid token: no user",
            )
        return DisplayUser.model_validate(payload)
    except PyJWTError:
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
    # This is a placeholder login that creates a token without validation
    token = create_access_token(
        {
            "username": username or str(uuid4()),
            "display_name": "Test",
            "password_hash": None,
        }
    )
    return UserStatus(token=token, ttl=TTL, is_new=True)


@app.get("/status", response_model=DisplayUser)
async def get_status(user: DisplayUser = Depends(get_current_user)):
    return user


@app.get("/room/{room}")
async def get(
    room: str,
    listen_seconds: int = Query(30, description="How long to listen in seconds"),
    user: DisplayUser = Depends(get_current_user),
):
    return StreamingResponse(
        get_message(room, user, timeout=listen_seconds),
        media_type="application/json",
    )


@app.post("/room/{room}")
async def send(
    room: str,
    message: ClientMessage,
    user: DisplayUser = Depends(get_current_user),
):
    msg = Message(user=user, message=message.message, timestamp=message.timestamp)
    await v.publish(room, msg.model_dump_json())  # type:ignore
    return {"message": f"send successful by user {user.username}"}


@app.post("/api/exit/{room}")
async def exit(room: str, user: DisplayUser = Depends(get_current_user)):
    if user.username == "anonymous":
        return {"message": "anonymous user exit does not affect subscriptions"}
    user_channel = f"user_exit:{user.username}"
    await v.publish(user_channel, STOPWORD)  # type:ignore
    return {"message": f"exit successful for user {user.username}"}


async def get_message(room: str, user: Optional[DisplayUser], timeout: int):
    async with v.pubsub() as pubsub:
        await pubsub.subscribe(room)  # type:ignore
        if user:
            await pubsub.subscribe(user.username)  # type:ignore
        if user and user.username != "anonymous":
            user_exit_channel = f"user_exit:{user.username}"
            await pubsub.subscribe(user_exit_channel)  # type:ignore

        end_time = asyncio.get_event_loop().time() + timeout

        while True:
            remaining = end_time - asyncio.get_event_loop().time()
            if remaining <= 0:
                yield b'{"event":"timeout"}'
                break
            try:
                message = await pubsub.get_message(  # type:ignore
                    ignore_subscribe_messages=True, timeout=remaining
                )
            except Exception:
                continue
            if message is not None:
                data: str | bytes = message["data"]  # type:ignore
                if isinstance(data, bytes):
                    data = data.decode()
                if data == STOPWORD:
                    break
                yield data.encode()  # if isinstance(data, str) else data
