from fastapi import APIRouter, WebSocket, WebSocketDisconnect

from app.datamodel.message import MessagePublic, MessageType, Plaintext
from app.datamodel.user import UserPublic
from app.dependencies import DatabaseDependencie, UserDependencie

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


@router.websocket("/rooms/{room}")
async def websocket_endpoint(
    websocket: WebSocket,
    room: str,
    user: UserDependencie,
    db_context: DatabaseDependencie,
):
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
