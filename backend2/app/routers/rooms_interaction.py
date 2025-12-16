import asyncio
from typing import Annotated, Any

from fastapi import APIRouter, Body, Query
from fastapi.responses import StreamingResponse
from sse_starlette.sse import EventSourceResponse

from app.datamodel.message import (
    MessagePublic,
    MessageSend,
    MessageType,
    Plaintext,
    SystemMessage,
)
from app.datamodel.user import UserPublic
from app.dependencies import (
    LEAVE_DELAY,
    DatabaseContext,
    DatabaseDependency,
    UserDependency,
)

router = APIRouter(
    prefix="/r",
    tags=["experimental", "rooms"],
)


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
    await db_context.valkey.publish(room, public_message.model_dump_json())

    return MessagePublic(
        type=MessageType.SYSTEM,
        content=Plaintext(content=f"send successful by user {user.username}"),
        sender=None,
    )


@router.get("/{room}")
async def listen(
    room: str,
    user: UserDependency,
    db_context: DatabaseDependency,
    listen_seconds: int = Query(30, description="How long to listen in seconds"),
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
