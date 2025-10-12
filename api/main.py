import asyncio
import json
import signal
from contextlib import asynccontextmanager
from datetime import datetime, timedelta
from typing import Annotated, Any, Optional
from uuid import uuid4

import jwt
import valkey.asyncio as valkey  # Assuming Valkey client works like this
from fastapi import Body, Depends, FastAPI, Header, HTTPException, Security, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import StreamingResponse
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from jwt import PyJWTError
from pydantic import BaseModel

TTL = 3600  # seconds

ALGORITHM = "HS256"
SECRET_KEY = "your-secret-key"  # Change this to a secure random key


auth = HTTPBearer()  # For enforcing authentication
bearer_scheme = HTTPBearer(auto_error=False)  # For optional auth

v = valkey.Valkey(host="localhost", port=6379, protocol=3)
TOKEN_PREFIX = "session_token:"

running = True

STOPWORD = "STOP"


def stop_server(*args):
    global running
    running = False


@asynccontextmanager
async def lifespan(app: FastAPI):
    signal.signal(signal.SIGINT, stop_server)
    yield
    await v.publish("*", STOPWORD)


app = FastAPI(lifespan=lifespan)

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


class Message(BaseModel):
    user: str
    message: str
    timestamp: datetime


class UserStatus(BaseModel):
    token: str
    ttl: int
    is_new: bool


fake_user_db = {
    "user1": "secret1",
    "user2": "secret2",
}


def create_access_token(data: dict, expires_delta: Optional[int] = None):
    to_encode = data.copy()
    expire = datetime.utcnow() + timedelta(seconds=expires_delta or TTL)
    to_encode.update({"exp": expire})
    token = jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)
    return token


async def get_current_user(
    credentials: Optional[HTTPAuthorizationCredentials] = Security(bearer_scheme),
):
    if credentials is None:
        # No token: treat as anonymous user
        return "anonymous"
    token = credentials.credentials
    try:
        payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
        user = payload.get("sub")
        if user is None:
            raise HTTPException(
                status_code=status.HTTP_401_UNAUTHORIZED,
                detail="Invalid token: no user",
            )
        return user
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
    # Issue anonymous token if no credentials provided
    if not username or not password:
        token = create_access_token({"sub": "anonymous"})
        return UserStatus(token=token, ttl=TTL, is_new=True)

    # Validate credentials
    user_password = fake_user_db.get(username)
    if not user_password or user_password != password:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid username or password",
        )

    token = create_access_token({"sub": username})
    return UserStatus(token=token, ttl=TTL, is_new=False)


@app.get("/api/r/{room}")
async def get(room: str, user: str = Depends(get_current_user)):
    return StreamingResponse(
        get_message(room, token=None if user == "anonymous" else user),
        media_type="application/json",
    )


@app.post("/api/r/{room}")
async def send(room: str, message: Message, user: str = Depends(get_current_user)):
    # Add user info to the message; optionally override message.user by user identity
    message.user = user
    await v.publish(room, message.model_dump_json())
    return {"message": f"send successful by user {user}"}


@app.post("/api/exit/{room}")
async def exit(room: str, user: str = Depends(get_current_user)):
    # Publish stopword to user-specific channel to only stop this user's listener
    if user == "anonymous":
        # Anonymous user may not have a stable unique identifier; optionally handle this case
        return {"message": "anonymous user exit does not affect subscriptions"}
    user_channel = f"user_exit:{user}"
    await v.publish(user_channel, STOPWORD)
    return {"message": f"exit successful for user {user}"}


def test(x):
    return x


async def get_message(room: str, token: Optional[str] = None):
    async with v.pubsub() as pubsub:
        await pubsub.subscribe(room)
        if token:
            await pubsub.subscribe(token)  # existing subscription to token channel
        if token and token != "anonymous":
            # Subscribe to user specific exit channel
            user_exit_channel = f"user_exit:{token}"
            await pubsub.subscribe(user_exit_channel)
        message: dict[str, Any] | None = None
        while running:
            try:
                message = await pubsub.get_message(
                    ignore_subscribe_messages=True, timeout=None
                )
            except Exception:
                pass
            if message is not None:
                data = message["data"].decode()
                if data == STOPWORD:
                    break
                yield message["data"]
