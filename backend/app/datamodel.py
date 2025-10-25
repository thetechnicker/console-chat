# import warnings
from enum import Enum  # , auto
from typing import Any, Optional
from app.database import DBPublicUser as PublicUser

from pydantic import BaseModel, model_validator
from typing_extensions import Self


class UserStatus(BaseModel):
    token: str
    ttl: int
    is_new: bool


class MessageType(Enum):
    TEXT = "TEXT"
    JOIN = "JOIN"
    LEAVE = "LEAVE"
    SYSTEM = "SYSTEM"
    # KEY = "KEY"


class BaseMessage(BaseModel):
    type: MessageType
    text: str
    data: Optional[dict[str, Any]] = None


class ClientMessage(BaseMessage):
    type: MessageType = MessageType.TEXT


class ServerMessage(BaseMessage):
    user: Optional[PublicUser] = None

    @model_validator(mode="after")
    def check_message_type_match(self) -> Self:
        if self.type != MessageType.SYSTEM and self.user is None:
            raise ValueError("A user must be specified for non-system messages.")
        return self
