import logging

from fastapi import APIRouter, HTTPException, WebSocket, WebSocketDisconnect, status
from sqlmodel import select

from app.datamodel.message import MessagePublic, MessageType, Plaintext, StaticRoom
from app.datamodel.user import User, UserPublic
from app.dependencies import DatabaseDependency, UserDependency

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
        await websocket.send_json(message)
        logger.debug(f"Sent personal message: {message}.")

    async def broadcast(self, room: str, message: MessagePublic):
        logger.debug(f"Broadcasting message to room '{room}': {message}.")
        for connection in self.active_connections.get(room, []):
            await connection.send_json(message)


manager = ConnectionManager()


@router.websocket("/room/{room}")
async def websocket_endpoint(
    websocket: WebSocket,
    room: str,
    user: UserDependency,
    db_context: DatabaseDependency,
):
    logger.info(f"Attempting to join room '{room}'.")

    stmt = select(StaticRoom).where(StaticRoom.name == room)
    db_room = db_context.psql_session.exec(stmt).one_or_none()

    if db_room:
        full_user = User.model_validate(user)
        logger.debug(f"User {full_user.username} is validated for room '{room}'.")
        if not (full_user.id == db_room.owner_id or full_user in db_room.users):
            logger.warning(
                f"Unauthorized access attempt by user {full_user.username} to room '{room}'."
            )
            raise HTTPException(status.HTTP_401_UNAUTHORIZED)
        await join_room(room, websocket, user)
    else:
        # logger.warning(f"Room '{room}' does not exist. Proceeding to join.")
        await join_room(room, websocket, user)


async def join_room(room: str, websocket: WebSocket, user: UserDependency):
    await manager.connect(room, websocket)
    public_user = UserPublic.model_validate(user)
    logger.debug(f"User {public_user.username} joined room '{room}'.")

    try:
        while True:
            message_json = await websocket.receive_json()
            logger.debug(
                f"Received message from user {public_user.username}: {message_json}."
            )
            message_json["sender"] = public_user
            message = MessagePublic.model_validate(message_json)
            await manager.send_personal_message(message, websocket)
            await manager.broadcast(room, message)
    except WebSocketDisconnect:
        manager.disconnect(room, websocket)
        logger.info(f"User {public_user.username} disconnected from room '{room}'.")
        await manager.broadcast(
            room,
            MessagePublic(
                type=MessageType.LEAVE,
                content=Plaintext(content=f"User {public_user.username} left"),
                sender=None,
            ),
        )
    except Exception as e:
        logger.error(
            f"Error occurred for user {public_user.username}: {str(e)}", exc_info=True
        )
