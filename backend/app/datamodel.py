import warnings
from enum import Enum  # , auto
from typing import Any, Optional
from pydantic import BaseModel, model_validator
from typing_extensions import Self

# ------------------------------------------------------------------------
#                                   API User
# ------------------------------------------------------------------------

# NOTE: only for development, once production ready, this will get removed and the logic is then forever fixed.
# This must be hardcoded!
OPT_IN_PRIVACY = False


class PublicUser(BaseModel):
    display_name: str
    # TODO: other public informations


class BetterUser(BaseModel):
    username: str
    password_hash: Optional[str]
    private: bool = False

    public_data: PublicUser

    def model_dump(self, **kwargs: Any):
        # Serialize full internal version by default (e.g., database)
        data = super().model_dump(**kwargs)
        if kwargs.get("db"):
            warnings.warn("THIS IS DANGEROUS")

        # WARNING: this might be dangerous
        if not kwargs.get("db"):
            # For public API responses, hide username when private is True
            # You might control this by passing `public=True` explicitly
            a = OPT_IN_PRIVACY and kwargs.get("public") is not None
            b = not OPT_IN_PRIVACY and kwargs.get("private") is None
            if (a or b) and data.get("private", not OPT_IN_PRIVACY):
                data.pop("username", None)

            # Always hide password_hash in any serialized output, expept for db
            data.pop("password_hash", None)

        # TODO: Deside if nesesery
        # else:
        #    pub = data.pop("public_data")
        #    data.update(pub)

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
    user: Optional[UserConfig] = None

    @model_validator(mode="after")
    def check_message_type_match(self) -> Self:
        if self.type != MessageType.SYSTEM and self.user is None:
            raise ValueError("A user must be specified for non-system messages.")
        return self
