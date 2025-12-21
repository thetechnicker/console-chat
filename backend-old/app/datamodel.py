# import warnings
from enum import Enum  # , auto
from typing import Any, Optional

from pydantic import BaseModel, model_validator
from typing_extensions import Self

from app.database import DBPublicUser as PublicUser


class UserStatus(BaseModel):
    token: str
    ttl: int
    is_new: bool


class MessageType(Enum):
    PLAIN_TEXT = "PLAIN-TEXT"
    ENCRYPTED_TEXT = "ENCRYPTED-TEXT"
    JOIN = "JOIN"
    LEAVE = "LEAVE"
    SYSTEM = "SYSTEM"
    KEY_REQUEST = "KEY-REQUEST"
    KEY = "KEY"


class BaseMessage(BaseModel):
    type: MessageType
    text: bytes | str
    data: Optional[dict[str, Any]] = None


class ClientMessage(BaseMessage):
    type: MessageType = MessageType.PLAIN_TEXT


class ServerMessage(BaseMessage):
    user: Optional[PublicUser] = None

    @model_validator(mode="after")
    def check_message_type_match(self) -> Self:
        if self.type != MessageType.SYSTEM and self.user is None:
            raise ValueError("A user must be specified for non-system messages.")
        return self
