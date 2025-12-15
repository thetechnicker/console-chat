import uuid
from datetime import datetime
from enum import StrEnum
from typing import List, Optional, Union

from fastapi import Body
from pydantic import BaseModel, Field, model_validator
from sqlmodel import JSON, Column, Field, Integer, Relationship, SQLModel
from typing_extensions import Self

from app.datamodel.user import User, UserPublic

type Json = dict[str, Json] | list[Json] | str | int | float | bool | None


class MessageType(StrEnum):
    ENCRYPTED = "ENCRYPTED"
    PLAINTEXT = "PLAINTEXT"
    KEY_REQUEST = "KEY_REQUEST"
    KEY_RESPONSE = "KEY_RESPONSE"
    SYSTEM = "SYSTEM"
    JOIN = "JOIN"
    LEAVE = "LEAVE"


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


def get_correct_message_type(message: MessageType):
    match message:
        case MessageType.ENCRYPTED:
            return Encrypted
        case MessageType.PLAINTEXT:
            return Plaintext
        case MessageType.KEY_REQUEST:
            return KeyRequest
        case MessageType.KEY_RESPONSE:
            return KeyResponse
        case MessageType.SYSTEM:
            return SystemMessage
        case MessageType.JOIN:
            return SystemMessage
        case MessageType.LEAVE:
            return SystemMessage


MessageContent = Union[Encrypted, Plaintext, KeyRequest, KeyResponse, SystemMessage]


class MessageBase(SQLModel):
    type: MessageType = Field(default=MessageType.PLAINTEXT, sa_column=Integer)
    content: Optional[MessageContent] = Field(default=None, sa_column=Column(JSON))
    send_at: datetime = Field(default_factory=datetime.now)
    data: Optional[Json] = Field(default=None, sa_column=Column(JSON))

    @model_validator(mode="after")
    def check_passwords_match(self) -> Self:
        if isinstance(self.content, get_correct_message_type(self.type)):
            return self
        raise ValueError(
            f"Wrong Message Type: Expected: {type(get_correct_message_type(self.type))}, got: {type(self.content)}"
        )


class MessageSend(MessageBase):
    pass


class MessagePublic(MessageBase):
    sender: Optional["UserPublic"]


class Message(MessageBase, table=True):
    id: int | None = Field(default=None, primary_key=True)
    sender_id: uuid.UUID = Field(foreign_key="user.id")
    sender: User = Relationship()  # link_model="message.sender_id")
    room_id: int = Field(foreign_key="staticroom.id")
    # receipient_id: uuid.UUID = Field(foreign_key="user.id")
    # receipient: User = Relationship(link_model="message.receipient_id")


class RoomLevel(StrEnum):
    FREE = "FREE"
    KEY = "KEY"
    INVITE_ONLY = "INVITE-ONLY"
    INVITE_AND_KEY = "INVITE-AND-KEY"


class StaticRoomUser(SQLModel, table=True):
    user_id: uuid.UUID | None = Field(
        default=None, foreign_key="user.id", primary_key=True
    )
    room_id: int | None = Field(
        default=None, foreign_key="staticroom.id", primary_key=True
    )


class RoomBase(SQLModel):
    name: str = Field(unique=True)
    key: str | None = Field(default=None)


class StaticRoom(RoomBase, table=True):
    id: int | None = Field(default=None, primary_key=True)
    owner_id: uuid.UUID | None = Field(default=None, foreign_key="user.id")
    owner: "User" = Relationship(back_populates="static_rooms")
    users: List["User"] = Relationship(link_model=StaticRoomUser)
    level: RoomLevel = Field(sa_column=Integer)


class StaticRoomPublic(RoomBase):
    id: int
    owner: "UserPublic"
    users: List["UserPublic"]
    level: RoomLevel = Field()


class CreateRoom(BaseModel):
    private_level: RoomLevel = Body()
    invite: Optional[list[uuid.UUID | str]] = Body(None)
    key: Optional[str] = Body(None)


class UpdateRoom(BaseModel):
    private_level: Optional[RoomLevel] = Body(None)
    invite: Optional[list[uuid.UUID | str]] = Body(None)
    key: Optional[str] = Body(None)
