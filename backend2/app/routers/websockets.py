from fastapi import APIRouter, HTTPException, WebSocket, WebSocketDisconnect, status
from sqlmodel import select

from app.datamodel.message import MessagePublic, MessageType, Plaintext, StaticRoom
from app.datamodel.user import User, UserPublic
from app.dependencies import DatabaseDependency, UserDependency

router = APIRouter(
    prefix="/ws",
    tags=["ws"],
)


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


@router.websocket("/room/{room}")
async def websocket_endpoint(
    websocket: WebSocket,
    room: str,
    user: UserDependency,
    db_context: DatabaseDependency,
):
    stmt = select(StaticRoom).where(StaticRoom.name == room)
    db_room = db_context.psql_session.exec(stmt).one_or_none()
    if db_room:
        full_user = User.model_validate(user)
        if not (full_user.id == db_room.owner_id or full_user in db_room.users):
            raise HTTPException(status.HTTP_401_UNAUTHORIZED)
        await join_room(room, websocket, user)
    else:
        await join_room(room, websocket, user)


async def join_room(room: str, websocket: WebSocket, user: UserDependency):
    await manager.connect(room, websocket)
    public_user = UserPublic.model_validate(user)
    try:
        while True:
            message_json = await websocket.receive_json()
            message_json["sender"] = public_user
            message = MessagePublic.model_validate(message_json)
            # await db_context.valkey.publish(room, message.model_dump_json())
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
