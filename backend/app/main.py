import asyncio
import hashlib
import os
import warnings
from datetime import datetime, timedelta, timezone
from typing import Any, Optional, Annotated, NamedTuple
from uuid import uuid4
from contextlib import asynccontextmanager

import jwt
import valkey.asyncio as valkey
from dotenv import load_dotenv
from fastapi import Body, Depends, FastAPI, HTTPException, Query, Security, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse, StreamingResponse
from fastapi.security import APIKeyHeader, HTTPAuthorizationCredentials, HTTPBearer
from jwt import PyJWTError
from sqlmodel import Session, select


# import sqlmodel
from pydantic import ValidationError

from app.database import (
    DBPublicUser,
    DBUser,
    init_postgesql_connection,
)
from app.datamodel import (
    ClientMessage,
    MessageType,
    ServerMessage,
    UserStatus,
)

BetterUser = DBUser
PublicUser = DBPublicUser

load_dotenv()

TTL = 60 * 60 * 24  # seconds
ALGORITHM = "HS256"
SECRET_KEY = os.getenv("SECRET", "secret")  # Secure random key recommended
if SECRET_KEY == "secret":
    warnings.warn("No secret given")

auth = HTTPBearer()  # Enforce auth
bearer_scheme = HTTPBearer(auto_error=False)  # Optional auth
api_key = APIKeyHeader(name="X-Api-Key")

TOKEN_PREFIX = "session_token:"

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
        # print(payload)
        pub_user = PublicUser.model_validate(payload["public_data"])
        user = BetterUser.model_validate(payload)
        user.public_data = pub_user
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


API_KEY_AUTH = Annotated[None, Depends(validate_api_key)]


@app.get("/")
async def root(_: API_KEY_AUTH):
    return {"message": "Hello World"}


@app.post("/auth", response_model=UserStatus)
async def login(
    username: Optional[str] = Body(None),
    password: Optional[str] = Body(None),
    context: DatabaseContext = Depends(get_db_context),
):
    password = hash_password(password) if password else password
    is_new = True
    if username and password:
        stmt = (
            select(DBUser)
            .where(DBUser.username == username)
            .where(DBUser.password_hash == password)
        )
        db_user = context.psql_session.exec(stmt).one_or_none()
        if db_user:
            is_new = False
            user = BetterUser.model_validate(db_user)
        else:
            raise HTTPException(
                status_code=status.HTTP_401_UNAUTHORIZED, detail="Wrong Credentials"
            )
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
    _: API_KEY_AUTH,
    context: DatabaseContext = Depends(get_db_context),
):
    try:
        # settings = v.get_connection_kwargs()  # type:ignore
        # print(settings)
        _ = await context.valkey.ping()
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
    context: DatabaseContext = Depends(get_db_context),
):
    try:
        # Check if already existing user
        stmt = select(DBUser).where(DBUser.username == user.username)
        db_user = context.psql_session.exec(stmt).one_or_none()
        if db_user:
            raise HTTPException(
                status_code=status.HTTP_405_METHOD_NOT_ALLOWED,
                detail="You are already registered",
            )
        # Overwrite user name and check for availability
        user.username = overwrite_username or user.username
        stmt = select(DBUser).where(DBUser.username == user.username)
        if context.psql_session.exec(stmt).one_or_none():
            raise HTTPException(
                status_code=status.HTTP_409_CONFLICT,
                detail=f"User {user.username} already exists",
            )

        # Hash password
        user.password_hash = hash_password(password)

        context.psql_session.add(user)
        context.psql_session.commit()
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
    context: DatabaseContext = Depends(get_db_context),
):
    msg = ServerMessage(
        user=user.public_data,
        text=message.text,
        # timestamp=datetime.now(timezone.utc),
        type=MessageType.TEXT,
    )
    await context.valkey.publish(
        room, msg.model_dump_json()
    )  # Use model_dump_json for serialization
    return {"message": f"send successful by user {user.public_data.display_name}"}


@app.get("/room/{room}", response_model=ServerMessage)
async def get(
    room: str,
    listen_seconds: int = Query(30, description="How long to listen in seconds"),
    user: BetterUser = Depends(get_current_user),
    context: DatabaseContext = Depends(get_db_context),
):
    await context.valkey.publish(
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
    context: DatabaseContext = Depends(get_db_context),
):
    async with context.valkey.pubsub() as pubsub:
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
