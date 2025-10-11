from typing import Annotated, Any
import signal
import json
import time
import asyncio
from contextlib import asynccontextmanager
from fastapi import FastAPI, HTTPException, Header, Depends
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from fastapi.responses import StreamingResponse
from fastapi.middleware.cors import CORSMiddleware
from uuid import uuid4
import asyncio
import valkey.asyncio as valkey
from pydantic import BaseModel
from datetime import datetime


TTL = 3600

v = valkey.Valkey(
    host="localhost", port=6379, protocol=3
)  # Assuming Valkey client can be instantiated like this
TOKEN_PREFIX = "session_token:"

running = True


def stop_server(*args):
    global running
    running = False


@asynccontextmanager
async def lifespan(app: FastAPI):
    signal.signal(signal.SIGINT, stop_server)
    yield
    await v.publish("*", STOPWORD)


app = FastAPI(lifespan=lifespan)

auth = HTTPBearer()

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


@app.get("/")
async def root():
    return {"message": "Hello World"}


@app.get("/api/status")
async def get_status(token: str | None = None) -> UserStatus:
    if token:
        exist = await v.get(TOKEN_PREFIX + token)
        if exist:
            await v.expire(TOKEN_PREFIX + token, TTL)
            return UserStatus(token=token, ttl=TTL, is_new=False)
    new_token = str(uuid4())
    await v.set(TOKEN_PREFIX + new_token, "valid", ex=TTL)
    return UserStatus(token=new_token, ttl=TTL, is_new=True)
    # return {"token": new_token, "ttl": TTL}


STOPWORD = "STOP"


async def get_message(room: str, token: str | None = None):
    async with v.pubsub() as pubsub:
        await pubsub.subscribe(room)
        if token:
            yield token
            await pubsub.subscribe(token)
        message: dict[str, Any] | None = None
        while running:
            try:
                message = await pubsub.get_message(
                    ignore_subscribe_messages=True, timeout=None
                )
            except:
                pass
            if message is not None:
                if message["data"].decode() == STOPWORD:
                    break
                yield message["data"]


@app.get("/api/r/{room}")
async def get(
    authorization: Annotated[HTTPAuthorizationCredentials, Depends(auth)], room: str
):
    return StreamingResponse(
        get_message(room, authorization.credentials), media_type="application/json"
    )


@app.post("/api/r/{room}")
async def send(
    authorization: Annotated[str, Depends(auth)], room: str, message: Message
):
    await v.publish(room, message.model_dump_json())
    return {"message": "send successful"}


@app.post("/api/exit/{room}")
async def exit(authorization: Annotated[str, Depends(auth)], room: str):
    await v.publish(room, STOPWORD)
    return {"message": "send successful"}
