import uuid
from enum import IntEnum
from typing import TYPE_CHECKING, List, Optional

from sqlmodel import Field, Relationship, SQLModel

if TYPE_CHECKING:
    from .message import StaticRoom


class UserType(IntEnum):
    GUEST = 1
    PERMANENT = 2


class UserBase(SQLModel):
    username: str = Field(default="anonym", unique=True, max_length=100)
    user_type: UserType = Field(default=UserType.GUEST)


class User(UserBase, table=True):
    id: uuid.UUID = Field(default_factory=uuid.uuid4, primary_key=True, index=True)
    password: Optional[str] = Field(default=None)  # use a hash in real applications
    appearance: "Appearance" = Relationship()
    appearance_id: int | None = Field(foreign_key="appearance.id", default=None)
    static_rooms: List["StaticRoom"] = Relationship(back_populates="owner")


class UserPublic(UserBase):
    appearance: "AppearancePublic"


class UserPrivate(UserPublic):
    id: uuid.UUID
    # password: Optional[str]


class UserUpdate(UserBase):
    password: str
    new_password: Optional[str]
    appearance: Optional["AppearanceUpdate"]


class AppearanceBase(SQLModel):
    color: str = Field(max_length=7, min_length=7)


class Appearance(AppearanceBase, table=True):
    id: int | None = Field(default=None, primary_key=True)


class AppearancePublic(AppearanceBase):
    pass


class AppearanceUpdate(UserBase):
    color: str
