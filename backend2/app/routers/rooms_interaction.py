import asyncio
import logging
import uuid
from datetime import UTC, datetime
from typing import Annotated

from fastapi import APIRouter, Body, Request
from sqlmodel import select
from sse_starlette.event import ServerSentEvent
from sse_starlette.sse import EventSourceResponse

from app.datamodel.message import *
from app.datamodel.user import *
from app.dependencies import DatabaseContext  # LEAVE_DELAY,
from app.dependencies import RESPONSES, DatabaseDependency, UserDependency

logger = logging.getLogger(__name__)
STREAM_DELAY = 1  # second
RETRY_TIMEOUT = 15000  # millisecond

router = APIRouter(
    prefix="/r",
    tags=["experimental", "rooms"],
)


@router.post("/static/{room}")
async def send_static(
    room: str,
    message: Annotated[MessageSend, Body()],
    user: UserDependency,
    db_context: DatabaseDependency,
):
    message_dict = message.model_dump()
    message_dict["sender"] = UserPublic.model_validate(user)
    public_message = MessagePublic.model_validate(message_dict)
    await _send(room, public_message, db_context, static_room=True)

    return public_message


@router.post("/{room}")
async def send(
    room: str,
    message: Annotated[MessageSend, Body()],
    user: UserDependency,
    db_context: DatabaseDependency,
):
    message_dict = message.model_dump()
    message_dict["sender"] = UserPublic.model_validate(user)
    public_message = MessagePublic.model_validate(message_dict)
    await _send(room, public_message, db_context)

    return public_message


async def _send(
    room: str,
    message: MessagePublic,
    db: DatabaseContext,
    static_room: bool = False,
    online: bool = False,
):
    if message.data is None:
        message.data = {"server_time": datetime.now(UTC).isoformat()}
    elif isinstance(message.data, dict):
        message.data["server_time"] = datetime.now(UTC).isoformat()

    if online:
        return
    await db.valkey.publish(room, message.model_dump_json())

    if static_room:
        db.psql_session.add(Message.model_validate(message))
        db.psql_session.commit()
    else:
        await db.valkey.lpush(room, message.model_dump_json())  # type:ignore
        await db.valkey.ltrim(room, 0, 10)  # type:ignore


@router.get(
    "/static/{room}",
    response_class=EventSourceResponse,
    status_code=200,
    responses={
        **RESPONSES,
        200: {
            "content": {
                "text/event-stream": {
                    "example": 'id: string\r\nevent: message\r\ndata: {"type":"PLAINTEXT","content":{"content":"string"},"send_at":"2025-12-17T17:11:54.282325Z","data":{"server_time":"2025-12-17T17:11:54.615735+00:00"},"sender":{"username":"TempUser_GhostlyTraveler_ej48ur","user_type":"GUEST","appearance":{"color":"#99adf3"}}}\r\nretry: 15000\r\n\r\n'
                }
            },
            "description": "event stream",
        },
    },
)
async def listen_static(
    room: str,
    user: UserDependency,
    db_context: DatabaseDependency,
    request: Request,
):
    _, num_users = (await db_context.valkey.pubsub_numsub(room))[0]
    await _send(
        room=room,
        message=MessagePublic(
            type=MessageType.JOIN,
            content=SystemMessage(
                content=f"User {user.username} joined the room", online_users=num_users
            ),
            sender=None,
        ),
        db=db_context,
        static_room=True,
        online=True,
    )
    return EventSourceResponse(
        event_generator(room, user, db_context, request, static_room=True),
    )


@router.get(
    "/{room}",
    response_class=EventSourceResponse,
    status_code=200,
    responses={
        **RESPONSES,
        200: {
            "content": {
                "text/event-stream": {
                    "example": 'id: string\r\nevent: message\r\ndata: {"type":"PLAINTEXT","content":{"content":"string"},"send_at":"2025-12-17T17:11:54.282325Z","data":{"server_time":"2025-12-17T17:11:54.615735+00:00"},"sender":{"username":"TempUser_GhostlyTraveler_ej48ur","user_type":"GUEST","appearance":{"color":"#99adf3"}}}\r\nretry: 15000\r\n\r\n'
                }
            },
            "description": "event stream",
        },
    },
)
async def listen(
    room: str,
    user: UserDependency,
    db_context: DatabaseDependency,
    request: Request,
):
    _, num_users = (await db_context.valkey.pubsub_numsub(room))[0]
    await _send(
        room=room,
        message=MessagePublic(
            type=MessageType.JOIN,
            content=SystemMessage(
                content=f"User {user.username} joined the room", online_users=num_users
            ),
            sender=None,
        ),
        db=db_context,
    )
    return EventSourceResponse(
        event_generator(room, user, db_context, request),
    )


async def event_generator(
    room: str,
    user: UserPrivate,
    db: DatabaseContext,
    request: Request,
    static_room: bool = False,
):
    if static_room:
        stmt = (
            select(Message)
            .where(Message.room.name == room)
            .order_by(Message.send_at)  # type:ignore
            .limit(10)
        )
        messages_json = await db.psql_session.exec(stmt)  # type: ignore
    else:
        messages_json = await db.valkey.lrange(room, 0, 10)  # type: ignore

    messages_json.reverse()
    for msg in [MessagePublic.model_validate_json(msg) for msg in messages_json]:
        event = ServerSentEvent(
            event="message",
            retry=RETRY_TIMEOUT,
            data=msg.model_dump_json(),
            id=str(uuid.uuid4()),
        )
        yield event
    async with db.valkey.pubsub() as pubsub:
        await pubsub.subscribe(room)
        while True:
            if await request.is_disconnected():
                break
            msg = await pubsub.get_message(ignore_subscribe_messages=True)
            if msg and msg["data"]:
                event = ServerSentEvent(
                    event="message",
                    retry=RETRY_TIMEOUT,
                    data=MessagePublic.model_validate_json(
                        msg["data"]
                    ).model_dump_json(),
                    id=str(uuid.uuid4()),
                )
                yield event
            await asyncio.sleep(STREAM_DELAY)
