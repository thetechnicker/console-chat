import asyncio
import uuid
from typing import Annotated

from fastapi import APIRouter, Body, Request
from sse_starlette.sse import EventSourceResponse

from app.datamodel.message import MessagePublic, MessageSend
from app.datamodel.user import UserPublic
from app.dependencies import DatabaseContext, DatabaseDependency, UserDependency

STREAM_DELAY = 1  # second
RETRY_TIMEOUT = 15000  # millisecond

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

    return public_message


@router.get("/{room}")
async def listen(
    room: str,
    user: UserDependency,
    db_context: DatabaseDependency,
    request: Request,
):
    return EventSourceResponse(
        event_generator(room, request, db_context),
    )


async def event_generator(
    room: str,
    request: Request,
    db: DatabaseContext,
):
    async with db.valkey.pubsub() as pubsub:
        await pubsub.subscribe(room)
        while True:
            if await request.is_disconnected():
                break
            msg = await pubsub.get_message(ignore_subscribe_messages=True)
            if msg and msg["data"]:
                event = {
                    "event": "new_message",
                    "retry": RETRY_TIMEOUT,
                    "data": MessagePublic.model_validate_json(
                        msg["data"]
                    ).model_dump_json(),
                    "id": str(uuid.uuid4()),
                }
                yield event
            await asyncio.sleep(STREAM_DELAY)
