import warnings
import os
from enum import Enum, auto
from typing import Any, Optional
from sqlalchemy import create_engine, Column, Integer, String, Boolean
from sqlalchemy.orm import declarative_base
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

        return data


# ------------------------------------------------------------------------
#                                    DB User
# ------------------------------------------------------------------------


Base = declarative_base()


class DBUser(Base):
    __tablename__ = "users"

    # User Identity
    id = Column(Integer, primary_key=True)
    username = Column(String, unique=True, nullable=False)
    password_hash = Column(String, nullable=False)
    private = Column(Boolean, default=True)

    # Customisation
    display_name = Column(String)


def create_user_db():
    user = os.getenv("POSTGRES_USER")
    password = os.getenv("POSTGRES_PASSWORD")
    host = "postgres"  # or actual host
    port = "5432"
    database = os.getenv("POSTGRES_DATABASE")

    connection_str = f"postgresql://{user}:{password}@{host}:{port}/{database}"
    engine = create_engine(connection_str)
    Base.metadata.create_all(engine)


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
