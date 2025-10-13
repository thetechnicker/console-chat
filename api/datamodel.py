from enum import Enum, auto
from typing import Any, Optional

from pydantic import BaseModel, model_validator
from typing_extensions import Self


class UserConfig(BaseModel):
    display_name: str


class User(BaseModel):
    # login
    username: str
    password_hash: str
    private: bool

    config: UserConfig


class UserStatus(BaseModel):
    token: str
    ttl: int
    is_new: bool


class MessageType(Enum):
    TEXT = auto()
    JOIN = auto()
    LEAVE = auto()
    SYSTEM = auto()


class Message(BaseModel):
    type: MessageType
    text: str
    data: Optional[dict[str, Any]] = None
    user: Optional[UserConfig] = None

    @model_validator(mode="after")
    def check_passwords_match(self) -> Self:
        if self.type != MessageType.SYSTEM and self.user is None:
            raise ValueError("A user must be specified for non-system messages.")
        return self
