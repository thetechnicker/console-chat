import warnings
from enum import Enum  # , auto
from typing import Any, Optional
from pydantic import BaseModel, model_validator
from typing_extensions import Self

# ------------------------------------------------------------------------
#                                   API User
# ------------------------------------------------------------------------


class PublicUser(BaseModel):
    display_name: str
    # TODO: other public informations


class BetterUser(BaseModel):
    username: str
    password_hash: Optional[str]
    private: bool = False

    public_data: PublicUser

    def model_dump(self, db: bool = False, **kwargs: Any):
        # Serialize full internal version by default (e.g., database)
        data = super().model_dump(**kwargs)

        # WARNING: this might be dangerous
        if db:
            warnings.warn("THIS IS DANGEROUS")
            data.pop("public_data", None)
        else:
            data["password_hash"] = None

        return data


# ------------------------------------------------------------------------
#                                    Other
# ------------------------------------------------------------------------


class UserStatus(BaseModel):
    token: str
    ttl: int
    is_new: bool


class UserConfig(BaseModel):
    display_name: str


class User(BaseModel):
    # login
    username: str
    password_hash: str
    private: bool

    config: UserConfig


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
