import asyncio
import hashlib
import json
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
from fastapi import (
    Body,
    Depends,
    FastAPI,
    HTTPException,
    Query,
    Security,
    WebSocket,
    WebSocketDisconnect,
    status,
)
from fastapi.exceptions import HTTPException
from fastapi.responses import StreamingResponse
from fastapi.security import APIKeyHeader, HTTPAuthorizationCredentials, HTTPBearer
from pydantic import BaseModel
from sqlmodel import Session, select

from app.datamodel import init_postgesql_connection
from app.datamodel.message import (  # Encrypted,; KeyRequest,; KeyResponse,; Message,; MessageContent,; StaticRoom,; StaticRoomPublic,
    MessagePublic,
    MessageSend,
    MessageType,
    Plaintext,
    SystemMessage,
)
from app.datamodel.user import AppearancePublic, User, UserPrivate, UserPublic


class Token(BaseModel):
    token: str
    ttl: int
    is_new: bool


class OnlineResponce(BaseModel):
    token: Token
    user: UserPrivate


class LoginData(BaseModel):
    username: str
    password: str


class UUIDEncoder(json.JSONEncoder):
    def default(self, o: Any):
        if isinstance(o, uuid.UUID):
            return str(o)  # Convert UUID to string
        return super().default(o)


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


SessionDep = Annotated[DatabaseContext, Depends(get_db_context)]


app = FastAPI(lifespan=lifespan)

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
    user: User | UserPrivate, expires_delta: int = TOKEN_TTL, is_new: bool = True
) -> Token:
    expire = datetime.now(timezone.utc) + timedelta(seconds=expires_delta or TOKEN_TTL)
    user_dict = UserPrivate.model_validate(user).model_dump()
    user_dict["id"] = str(user_dict["id"])
    to_encode = {
        "exp": expire,
        "iss": "http://localhost:8000/auth",
        "user": user_dict,
    }
    token_str = jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)
    token = Token(token=token_str, ttl=expires_delta, is_new=is_new)
    return token


def get_user_from_token(token: str) -> UserPrivate:
    try:
        # Decode the JWT token
        payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
        return UserPrivate.model_validate(payload.get("user"))  # Adjust as needed
    except jwt.ExpiredSignatureError:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Token has expired"
        )
    except jwt.PyJWTError:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid token"
        )


def get_current_user(
    credentials: HTTPAuthorizationCredentials = Security(bearer_scheme),
) -> UserPrivate:
    return get_user_from_token(credentials.credentials)


def validate_api_key(key: str = Depends(api_key)):
    dest_key = os.environ.get("DEV_API_KEY")
    if dest_key and dest_key == key:
        return
    raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED)


API_KEY_AUTH = Annotated[None, Depends(validate_api_key)]


@app.get("/online", response_model=OnlineResponce)
def online(
    credentials: Optional[HTTPAuthorizationCredentials] = Security(bearer_scheme),
    db_context: DatabaseContext = Depends(get_db_context),
):
    # Handle Bearer Token Authentication
    if credentials:
        user = get_user_from_token(credentials.credentials)
        if user:
            token = create_access_token(user, TOKEN_TTL)
            return OnlineResponce(token=token, user=user)

    id = uuid.uuid4()
    user = UserPrivate(
        id=id,
        appearance=AppearancePublic(color=deterministic_color_from_string(str(id))),
    )
    token = create_access_token(user, TOKEN_TTL, True)
    return OnlineResponce(token=token, user=user)


@app.post("/login", response_model=OnlineResponce)
def login(
    login: Annotated[LoginData, Body()],
    db_context: DatabaseContext = Depends(get_db_context),
):
    # Handle Username and Password Authentication
    stmt = select(User).where(User.username == login.username)
    user = db_context.psql_session.exec(stmt).one_or_none()
    if (
        user
        and user.password
        and verify_password(user.password, login.username, login.password)
    ):
        token = create_access_token(user, TOKEN_TTL, True)
        return OnlineResponce(token=token, user=UserPrivate.model_validate(user))
    else:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid credentials"
        )


@app.post("/room/{room}")
async def send(
    room: str,
    message: Annotated[MessageSend, Body()],
    user: UserPrivate = Depends(get_current_user),
    db_context: DatabaseContext = Depends(get_db_context),
):
    message_dict = message.model_dump()
    message_dict["sender"] = UserPublic.model_validate(user)
    public_message = MessagePublic.model_validate(message_dict)
    await db_context.valkey.publish(room, public_message.model_dump_json())

    return MessagePublic(
        type=MessageType.SYSTEM,
        content=Plaintext(content=f"send successful by user {user.username}"),
        sender=None,
    )


@app.get("/room/{room}")
async def listen(
    room: str,
    listen_seconds: int = Query(30, description="How long to listen in seconds"),
    user: UserPrivate = Depends(get_current_user),
    db_context: DatabaseContext = Depends(get_db_context),
):
    first_join = await db_context.valkey.exists(f"{room}:{user.username}") == 0
    if first_join:
        # _, num_users = (await db_context.valkey.pubsub_numsub(room))[0]
        await db_context.valkey.publish(
            room,
            MessagePublic(
                type=MessageType.JOIN,
                content=Plaintext(content=f"User {user.username} joined"),
                sender=user,
            ).model_dump_json(),
        )
        await db_context.valkey.set(
            f"{room}:{user.username}", "1", ex=listen_seconds + LEAVE_DELAY
        )
    else:
        await db_context.valkey.expire(
            f"{room}:{user.username}", listen_seconds + LEAVE_DELAY
        )
    return StreamingResponse(
        get_message(room, listen_seconds, db_context, first_join),
        media_type="application/json",
    )


async def get_message(
    room: str,
    timeout: int,
    context: DatabaseContext,
    first_join: bool = False,
):
    async with context.valkey.pubsub() as pubsub:
        await pubsub.subscribe(room)
        if first_join:
            _, num_users = (await context.valkey.pubsub_numsub(room))[0]
            yield MessagePublic(
                type=MessageType.SYSTEM,
                content=SystemMessage(content="People Online", online_users=num_users),
                sender=None,
            ).model_dump_json().encode()

        end_time = asyncio.get_event_loop().time() + timeout

        while True:
            remaining = end_time - asyncio.get_event_loop().time()
            if remaining <= 0:
                yield b"END"
                break
            try:
                message = await pubsub.get_message(
                    ignore_subscribe_messages=True, timeout=remaining
                )
            except Exception:
                continue
            if message is not None:
                data: str | bytes | Any = message["data"]
                if not isinstance(data, str):
                    print(type(data))
                yield data


class ConnectionManager:
    def __init__(self):
        self.active_connections: dict[str, list[WebSocket]] = {}

    async def connect(self, room: str, websocket: WebSocket):
        await websocket.accept()
        self.active_connections.setdefault(room, [])
        self.active_connections[room].append(websocket)

    def disconnect(self, room: str, websocket: WebSocket):
        if room in self.active_connections:
            self.active_connections[room].remove(websocket)

    async def send_personal_message(self, message: MessagePublic, websocket: WebSocket):
        await websocket.send_json(message)

    async def broadcast(self, room: str, message: MessagePublic):
        for connection in self.active_connections.get(room, []):
            await connection.send_json(message)


manager = ConnectionManager()


@app.websocket("/ws/room/{room}")
async def websocket_endpoint(
    websocket: WebSocket,
    room: str,
    user: UserPrivate = Depends(get_current_user),
    db_context: DatabaseContext = Depends(get_db_context),
):
    await manager.connect(room, websocket)
    public_user = UserPublic.model_validate(user)
    try:
        while True:
            message_json = await websocket.receive_json()
            message_json["sender"] = public_user
            message = MessagePublic.model_validate(message_json)
            # await db_context.valkey.publish(room, public_message.model_dump_json())
            await manager.send_personal_message(message, websocket)
            await manager.broadcast(room, message)
    except WebSocketDisconnect:
        manager.disconnect(room, websocket)
        await manager.broadcast(
            room,
            MessagePublic(
                type=MessageType.LEAVE,
                content=Plaintext(content=f"User {public_user.username} left"),
                sender=None,
            ),
        )
