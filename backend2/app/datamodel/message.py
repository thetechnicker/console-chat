import uuid
from datetime import datetime
from enum import IntEnum
from typing import List, Optional, Union

from pydantic import BaseModel
from sqlmodel import JSON, Column, Field, Relationship, SQLModel

from app.datamodel.user import User, UserPublic

type Json = dict[str, Json] | list[Json] | str | int | float | bool | None


class MessageType(IntEnum):
    ENCRYPTED = 1
    PLAINTEXT = 2
    KEY_REQUEST = 3
    KEY_RESPONSE = 4
    SYSTEM = 5
    JOIN = 6
    LEAVE = 7


class BaseMessage(BaseModel):
    pass


class Encrypted(BaseMessage):
    content_base64: str
    nonce: str


class Plaintext(BaseMessage):
    content: str


class KeyRequest(BaseMessage):
    public_key: str


class KeyResponse(BaseMessage):
    encrypted_symmetric_key: str
    check_msg: str
    sender_public_key: str


class SystemMessage(BaseMessage):
    content: str
    online_users: int


class JoinMessage(BaseMessage):
    content: str


MessageContent = Union[Encrypted, Plaintext, KeyRequest, KeyResponse, SystemMessage]


class MessageBase(BaseModel):
    type: MessageType = Field(default=MessageType.PLAINTEXT)
    content: Optional[MessageContent] = Field(default=None, sa_column=Column(JSON))
    send_at: datetime = Field(default_factory=datetime.now)
    data: Optional[Json] = Field(default=None, sa_column=Column(JSON))


class MessageSend(MessageBase):
    pass


class MessagePublic(MessageBase):
    sender: Optional["UserPublic"]


class RoomBase(SQLModel):
    name: str = Field(unique=True)
    key: str = Field()


class StaticRoomUser(SQLModel, table=True):
    user_id: uuid.UUID | None = Field(
        default=None, foreign_key="user.id", primary_key=True
    )
    room_id: int | None = Field(
        default=None, foreign_key="staticroom.id", primary_key=True
    )


class StaticRoom(RoomBase, table=True):
    id: int = Field(primary_key=True)
    owner_id: uuid.UUID = Field(default=None, foreign_key="user.id")
    owner: "User" = Relationship(back_populates="static_rooms")
    users: List["User"] = Relationship(link_model=StaticRoomUser)


class StaticRoomPublic(RoomBase):
    id: int
    owner: "User"
