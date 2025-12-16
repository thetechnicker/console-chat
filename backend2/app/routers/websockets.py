import logging
from contextlib import asynccontextmanager
from typing import cast

from fastapi import (
    APIRouter,
    Request,
    WebSocket,
    WebSocketDisconnect,
    WebSocketException,
    status,
)
from sqlmodel import select
from valkey.asyncio import Valkey

from app.datamodel.message import (
    MessagePublic,
    MessageType,
    StaticRoom,
    StaticRoomPublic,
    SystemMessage,
)
from app.datamodel.user import User, UserPublic
from app.dependencies import (
    DatabaseDependency,
    auth_bearer_scheme,
    get_current_permanent_user,
    get_current_user,
    get_db_context,
)

logger = logging.getLogger(__name__)

manager = None


@asynccontextmanager
async def init_manager(router: APIRouter):
    """
    Initialize the connection manager for WebSocket routes.

    Creates a database context for the router and initializes
    the connection manager before yielding control to the router.
    """
    logger.debug(f"Initializing connection manager.")
    global manager
    async with asynccontextmanager(get_db_context)() as db:
        manager = ConnectionManager(db.valkey)
        yield


router = APIRouter(
    lifespan=init_manager,
    # deprecated=True,
    prefix="/ws",
    tags=["ws"],
)


class ConnectionManager:
    def __init__(self, valkey_instance: Valkey):
        """
        Manages active WebSocket connections.

        Args:
            valkey_instance (Valkey): An instance of Valkey for message handling.
        """
        self.active_connections: dict[str, list[WebSocket]] = {}
        self.valkey: None | Valkey = valkey_instance

    async def add_message_to_buffer(self, room: str, message: MessagePublic):
        """Add a message to the Valkey buffer for the specified room."""
        await self.valkey.rpush(room, message.model_dump_json())  # type: ignore

    async def get_messages_from_buffer(self, room: str, limit: int = 100):
        """Retrieve messages from the Valkey buffer for the specified room."""
        messages_json = await self.valkey.lrange(room, 0, limit)  # type: ignore
        return [MessagePublic.model_validate_json(msg) for msg in messages_json]

    async def connect(self, room: str, websocket: WebSocket):
        """
        Accept a WebSocket connection and register it.

        Args:
            room (str): The room name to which the user is connecting.
            websocket (WebSocket): The WebSocket connection to be registered.
        """
        await websocket.accept()
        logger.debug(f"WebSocket connection established for room '{room}'.")
        self.active_connections.setdefault(room, [])
        self.active_connections[room].append(websocket)

    def disconnect(self, room: str, websocket: WebSocket):
        """Disconnect a WebSocket connection from the specified room."""
        if room in self.active_connections:
            self.active_connections[room].remove(websocket)
            logger.debug(f"WebSocket disconnected from room '{room}'.")

    async def send_personal_message(self, message: MessagePublic, websocket: WebSocket):
        """Send a personal message to the specified WebSocket connection."""
        await websocket.send_text(message.model_dump_json())
        logger.debug(f"Sent personal message: {message}.")

    async def broadcast(self, room: str, message: MessagePublic):
        """Broadcast a message to all connections in the specified room."""
        logger.debug(f"Broadcasting message to room '{room}': {message}.")
        for connection in self.active_connections.get(room, []):
            await connection.send_text(message.model_dump_json())

    def get_num_connected(self, room: str):
        """Return the number of active connections for the specified room."""
        return len(self.active_connections.get(room, []))


@router.websocket("/room/{username}/{room}")
async def static_room(
    websocket: WebSocket,
    username: str,
    room: str,
    db_context: DatabaseDependency,
):
    logger.debug(f"Attempting to join a temporary room '{room}'.")

    token = await auth_bearer_scheme(cast(Request, websocket))
    if not token:
        raise WebSocketException(status.HTTP_401_UNAUTHORIZED)

    user = await get_current_permanent_user(token, db_context)
    public_user = UserPublic.model_validate(user)

    stmt = select(User).where(User.username == username)
    owner = db_context.psql_session.exec(stmt).one_or_none()

    if not owner:
        raise WebSocketException(status.HTTP_404_NOT_FOUND)

    stmt = (
        select(StaticRoom)
        .where(StaticRoom.owner_id == owner.id)
        .where(StaticRoom.name == room)
    )
    db_room = db_context.psql_session.exec(stmt).one_or_none()
    if not db_room:
        raise WebSocketException(status.HTTP_404_NOT_FOUND)

    logger.debug(f"User {user.username} is validated for room '{room}'.")

    public_room = StaticRoomPublic.model_validate(db_room)
    if not (user.id == db_room.owner_id or public_user in public_room.users):
        logger.warning(
            f"Unauthorized access attempt by user {user.username} to room '{room}'."
        )
        raise WebSocketException(status.HTTP_401_UNAUTHORIZED)
    await join_room(room, websocket, public_user)


@router.websocket("/room/{room}")
async def temporary_room(
    websocket: WebSocket,
    room: str,
    db_context: DatabaseDependency,
):
    logger.debug(f"Attempting to join a temporary room '{room}'.")

    token = await auth_bearer_scheme(cast(Request, websocket))
    if not token:
        raise WebSocketException(status.HTTP_401_UNAUTHORIZED)

    user = await get_current_user(token, db_context)
    public_user = UserPublic.model_validate(user)

    await join_room(room, websocket, public_user)


async def join_room(
    room: str,
    websocket: WebSocket,
    user: UserPublic,
):
    """
    Join a user to a specified room and send previous messages.

    Args:
        room (str): The room to join.
        websocket (WebSocket): The WebSocket connection for the user.
        user (UserPublic): The public user information for the joining user.

    This function establishes the WebSocket connection, retrieves previous messages
    from the room, and broadcasts a join message to all users in that room.
    """
    if manager is None:
        raise WebSocketException(status.HTTP_500_INTERNAL_SERVER_ERROR)

    await manager.connect(room, websocket)
    previous_messages = await manager.get_messages_from_buffer(room)

    for msg in previous_messages:
        await manager.send_personal_message(msg, websocket)

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
        logger.debug(f"User {user.username} disconnected from room '{room}'.")

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
