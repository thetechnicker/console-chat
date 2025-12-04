import os
import uuid
from datetime import datetime
from enum import IntEnum
from typing import List, Optional, Union

from pydantic import BaseModel
from sqlmodel import JSON, Column, Field, Relationship, SQLModel, create_engine

type Json = dict[str, Json] | list[Json] | str | int | float | bool | None


def set_connection_str(host: Optional[str] = None):
    user = os.getenv("POSTGRES_USER")
    password = os.getenv("POSTGRES_PASSWORD")
    if host is None:
        host = os.getenv("POSTGRES_HOST", "postgres")
    port = "5432"
    database = os.getenv("POSTGRES_DB")
    return f"postgresql://{user}:{password}@{host}:{port}/{database}"


class UserType(IntEnum):
    GUEST = 1
    PERMANENT = 2


class User(SQLModel, table=True):
    id: uuid.UUID = Field(default_factory=uuid.uuid4, primary_key=True, index=True)

    username: str = Field(default="anonym", unique=True, max_length=100)
    password: Optional[str] = Field(default=None)  # use a hash in real applications

    user_type: UserType = Field(default=UserType.GUEST)

    appearance_id: int = Field(foreign_key="appearance.id")
    appearance: "Appearance" = Relationship(
        back_populates="user",
    )

    static_rooms: List["StaticRoom"] = Relationship(back_populates="owner")
    # messages: List["Message"] = Relationship(back_populates="sender")


class Appearance(SQLModel, table=True):
    id: int = Field(primary_key=True, index=True)
    # user_id: uuid.UUID = Field(default=None, foreign_key="user.id")
    user: User = Relationship(back_populates="appearance")
    color: Optional[str] = Field(default=None)


class MessageType(IntEnum):
    ENCRYPTED = 1
    PLAINTEXT = 2
    KEY_REQUEST = 3
    KEY_RESPONSE = 4
    SYSTEM = 5


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


# type MessageContent = Encrypted | Plaintext | KeyRequest | KeyResponse | SystemMessage
MessageContent = Union[Encrypted, Plaintext, KeyRequest, KeyResponse, SystemMessage]


class Message(SQLModel, table=True):
    id: int = Field(primary_key=True)
    room: Optional[str] = Field(default=None, foreign_key="staticroom.name")
    # static_room: Opt = Field(default=None, foreign_key="staticroom.id")
    sender_id: Optional[uuid.UUID] = Field(default=None, foreign_key="user.id")
    sender: Optional[User] = Relationship()

    type: MessageType = Field(default=MessageType.PLAINTEXT)
    content: Optional[MessageContent] = Field(default=None, sa_column=Column(JSON))
    send_at: datetime = Field(
        default_factory=datetime.now
    )  # Consider using a specific type like datetime
    data: Optional[Json] = Field(default=None, sa_column=Column(JSON))


class StaticRoom(SQLModel, table=True):
    id: int = Field(primary_key=True)
    name: str = Field(unique=True)
    key: str = Field()
    owner_id: uuid.UUID = Field(default=None, foreign_key="user.id")
    owner: User = Relationship(back_populates="static_rooms")
    messages: List[Message] = Relationship()


engine = None


def init_postgesql_connection():
    global engine
    connection_str: str = set_connection_str()
    engine = create_engine(
        connection_str,
        pool_size=20,
        max_overflow=10,
        pool_recycle=3600,
        pool_timeout=30,
    )
    SQLModel.metadata.create_all(engine)
    return engine
