import logging
from typing import Annotated, cast

from fastapi import (
    APIRouter,
    Cookie,
    HTTPException,
    Query,
    Request,
    WebSocket,
    WebSocketDisconnect,
    WebSocketException,
    status,
)
from sqlmodel import select

from app.datamodel.message import MessagePublic, MessageType, StaticRoom, SystemMessage
from app.datamodel.user import User, UserPublic
from app.dependencies import (
    DatabaseContext,
    DatabaseDependency,
    auth_bearer_scheme,
    get_current_user,
)

router = APIRouter(
    prefix="/ws",
    tags=["ws"],
)

logger = logging.getLogger(__name__)


class ConnectionManager:
    def __init__(self):
        self.active_connections: dict[str, list[WebSocket]] = {}

    async def connect(self, room: str, websocket: WebSocket):
        await websocket.accept()
        logger.info(f"WebSocket connection established for room '{room}'.")
        self.active_connections.setdefault(room, [])
        self.active_connections[room].append(websocket)

    def disconnect(self, room: str, websocket: WebSocket):
        if room in self.active_connections:
            self.active_connections[room].remove(websocket)
            logger.info(f"WebSocket disconnected from room '{room}'.")

    async def send_personal_message(self, message: MessagePublic, websocket: WebSocket):
        await websocket.send_text(message.model_dump_json())
        logger.debug(f"Sent personal message: {message}.")

    async def broadcast(self, room: str, message: MessagePublic):
        logger.debug(f"Broadcasting message to room '{room}': {message}.")
        for connection in self.active_connections.get(room, []):
            await connection.send_text(message.model_dump_json())

    def get_num_connected(self, room: str):
        return len(self.active_connections.get(room, []))


manager = ConnectionManager()


async def get_cookie_or_token(
    websocket: WebSocket,
    session: Annotated[str | None, Cookie()] = None,
    token: Annotated[str | None, Query()] = None,
):
    if session is None and token is None:
        raise WebSocketException(code=status.WS_1008_POLICY_VIOLATION)
    return session or token


@router.get("/room")
async def coockie_test(id: Annotated[str | None, Cookie()] = None):
    return id


@router.websocket("/room")
async def listen_room():
    pass


@router.websocket("/room/{room}")
async def websocket_endpoint(
    websocket: WebSocket,
    room: str,
    db_context: DatabaseDependency,
):
    logger.info(f"Attempting to join room '{room}'.")

    stmt = select(StaticRoom).where(StaticRoom.name == room)
    db_room = db_context.psql_session.exec(stmt).one_or_none()

    token = await auth_bearer_scheme(cast(Request, websocket))
    if not token:
        raise HTTPException(status.HTTP_401_UNAUTHORIZED)
    user = await get_current_user(token, db_context)

    public_user = UserPublic.model_validate(user)
    if db_room:
        full_user = User.model_validate(user)
        logger.debug(f"User {full_user.username} is validated for room '{room}'.")
        if not (full_user.id == db_room.owner_id or full_user in db_room.users):
            logger.warning(
                f"Unauthorized access attempt by user {full_user.username} to room '{room}'."
            )
            raise HTTPException(status.HTTP_401_UNAUTHORIZED)
        await join_room(room, websocket, public_user)
    else:
        await join_room(room, websocket, public_user)


async def join_room(
    room: str,
    websocket: WebSocket,
    user: UserPublic,
):
    await manager.connect(room, websocket)
    await manager.broadcast(
        room,
        MessagePublic(
            type=MessageType.JOIN,
            content=SystemMessage(
                content=f"User {user.username} joined",
                online_users=manager.get_num_connected(room),
            ),
            sender=None,
        ),
    )
    logger.debug(f"User {user.username} joined room '{room}'.")
    try:
        while True:
            message_json = await websocket.receive_json()

            logger.debug(f"Received message from user {user.username}: {message_json}.")
            message_json["sender"] = user
            message = MessagePublic.model_validate(message_json)
            await manager.send_personal_message(message, websocket)
            await manager.broadcast(room, message)
    except WebSocketDisconnect:
        manager.disconnect(room, websocket)
        logger.info(f"User {user.username} disconnected from room '{room}'.")
        await manager.broadcast(
            room,
            MessagePublic(
                type=MessageType.LEAVE,
                content=SystemMessage(
                    content=f"User {user.username} left",
                    online_users=manager.get_num_connected(room),
                ),
                sender=None,
            ),
        )
    except Exception as e:
        logger.error(
            f"Error occurred for user {user.username}: {str(e)}", exc_info=True
        )
