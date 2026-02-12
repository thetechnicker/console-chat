import asyncio
import logging
import uuid
from datetime import UTC, datetime
from typing import Annotated

from fastapi import APIRouter, Body, HTTPException, Request, status
from sqlmodel import and_, col, or_, select
from sse_starlette.event import ServerSentEvent
from sse_starlette.sse import EventSourceResponse

from app.datamodel import SYSTEM_USER
from app.datamodel.message import *
from app.datamodel.user import *
from app.dependencies import DatabaseContext  # LEAVE_DELAY,
from app.dependencies import RESPONSES, DatabaseDependency, UserDependency

logger = logging.getLogger(__name__)
STREAM_DELAY = 1  # second
RETRY_TIMEOUT = 15000  # millisecond

router = APIRouter(
    prefix="/room",
    tags=["rooms", "sse"],
)


@router.post("/static/{room}", responses={**RESPONSES, 200: {"model": MessagePublic}})
async def send_static(
    room: str,
    message: Annotated[MessageSend, Body()],
    user: UserDependency,
    db_context: DatabaseDependency,
):
    message_dict = message.model_dump()
    message_dict["sender"] = UserPublic.model_validate(user)
    public_message = MessagePublic.model_validate(message_dict)
    await _send(room, public_message, db_context, user, static_room=True)

    return public_message


@router.post("/{room}", responses={**RESPONSES, 200: {"model": MessagePublic}})
async def send(
    room: str,
    message: Annotated[MessageSend, Body()],
    user: UserDependency,
    db_context: DatabaseDependency,
):
    message_dict = message.model_dump()
    message_dict["sender"] = UserPublic.model_validate(user)
    public_message = MessagePublic.model_validate(message_dict)
    await _send(room, public_message, db_context, user)

    return public_message


async def _send(
    room: str,
    message: MessagePublic,
    db: DatabaseContext,
    user: UserPrivate,
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
        stmt = select(StaticRoom).where(StaticRoom.name == room)
        db_room = db.psql_session.exec(stmt).unique().one_or_none()
        if db_room is None:
            raise HTTPException(
                status_code=status.HTTP_401_UNAUTHORIZED,
                detail="This room is offlimits for you",
            )
        msg = Message.model_validate(
            {**message.model_dump(), "sender_id": user.id, "room_id": db_room.id}
        )
        msg.content = msg.content.model_dump()  # type: ignore
        db.psql_session.add(msg)
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
    db: DatabaseDependency,
    request: Request,
):
    _, num_users = (await db.valkey.pubsub_numsub(room))[0]
    stmt = (
        select(StaticRoom)
        .where(StaticRoom.name == room)
        .outerjoin(StaticRoomUser)
        .where(
            or_(
                StaticRoom.owner_id == user.id,
                and_(
                    StaticRoomUser.room_id == StaticRoom.id,
                    StaticRoomUser.user_id == user.id,
                ),
            )
        )
    )
    room_res = db.psql_session.exec(stmt)  # type: ignore
    db_room = room_res.one_or_none()
    if db_room is None:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="You are not allowed to join",
        )

    await _send(
        room=room,
        message=MessagePublic(
            type=MessageType.JOIN,
            content=SystemMessage(
                content=f"User {user.username} joined the room", online_users=num_users
            ),
            send_at=datetime.now(),
            sender=UserPublic.model_validate(SYSTEM_USER),
        ),
        db=db,
        user=user,
        static_room=True,
        online=True,
    )
    return EventSourceResponse(
        event_generator(room, user, db, request, static_room=True, room_id=db_room.id),
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
            send_at=datetime.now(),
            sender=UserPublic.model_validate(SYSTEM_USER),
        ),
        user=user,
        db=db_context,
    )
    return EventSourceResponse(
        event_generator(room, user, db_context, request),
    )


async def save_event_generator(
    room: str,
    user: UserPrivate,
    db: DatabaseContext,
    request: Request,
    static_room: bool = False,
):
    try:
        async for event in event_generator(room, user, db, request, static_room):
            yield event
    except:
        logger.error(
            f"Error while listening to room: {room}", exc_info=True, stack_info=True
        )


async def event_generator(
    room: str,
    user: UserPrivate,
    db: DatabaseContext,
    request: Request,
    static_room: bool = False,
    room_id: None | int = None,
):
    if static_room and room_id:
        stmt = (
            select(Message)
            .where(Message.room_id == room_id)
            .order_by(col(Message.send_at))
            .limit(10)
        )
        messages_json = list(room.model_dump_json() for room in db.psql_session.exec(stmt).all())  # type: ignore
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
