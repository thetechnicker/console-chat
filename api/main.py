from typing import Annotated, Any
from fastapi import FastAPI, HTTPException, Header
from uuid import uuid4
from valkey import Valkey
from pydantic import BaseModel


class Message(BaseModel):
    user: str
    message: str
    timestamp: str


valkey = Valkey(
    host="localhost", port=6379, protocol=3
)  # Assuming Valkey client can be instantiated like this
valkey.set("on", 1)
p = valkey.pubsub()
# p.subscribe()

app = FastAPI()
TOKEN_PREFIX = "session_token:"


@app.get("/")
async def root():
    return {"message": "Hello World"}


@app.get("/api/status")
async def get_status():
    # Generate new token and store it with a TTL (e.g., 1 hour)
    new_token = str(uuid4())
    valkey.set(TOKEN_PREFIX + new_token, "valid", ex=3600)
    return {"token": new_token}


@app.get("/api/r/{room}")
async def get(token: Annotated[str, Header()], room: str):
    exists = valkey.exists(TOKEN_PREFIX + token)
    if not exists:
        raise HTTPException(status_code=401, detail="Invalid or missing session token")
    # Proceed with processing the message
    return {"message": "hi and welcome", "msg": lastmsg["data"]}


@app.post("/api/r/{room}")
async def send(token: Annotated[str, Header()], room: str, message: Message):
    exists = valkey.exists(TOKEN_PREFIX + token)
    if not exists:
        raise HTTPException(status_code=401, detail="Invalid or missing session token")
    # Proceed with processing the message
    valkey.publish("rooma", message.message)
    return {"message": "send successful", "m": message}
