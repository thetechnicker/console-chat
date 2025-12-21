import random
import string
import uuid
from enum import StrEnum
from typing import TYPE_CHECKING, List, Optional

from sqlmodel import Field, Integer, Relationship, SQLModel

if TYPE_CHECKING:
    from .message import StaticRoom


def random_suffix(length: int = 5, rand: Optional[random.Random] = None):
    """Generate a random alphanumeric string deterministically if rand is provided."""
    if rand is None:
        rand = random.Random()
    return "".join(rand.choices(string.ascii_lowercase + string.digits, k=length))


def generate_temp_username(id: Optional[str | uuid.UUID] = None):
    prefixes = [
        "TempUser",
        "Guest",
        "Anon",
        "Visitor",
        "Unreg",
        "Phantom",
        "Shadow",
        "Ephemeral",
    ]
    adjectives = [
        "Swift",
        "Silent",
        "Wandering",
        "Hidden",
        "Transient",
        "Fleeting",
        "Passing",
        "Ghostly",
    ]
    nouns = [
        "Nomad",
        "Specter",
        "Voyager",
        "Traveler",
        "Entity",
        "Drifter",
        "Stranger",
        "Wisp",
    ]
    uuid_str = str(id) if id else None
    if uuid_str is not None:
        rand = random.Random(uuid_str)
    else:
        rand = random.Random()

    prefix = rand.choice(prefixes)
    adj = rand.choice(adjectives)
    noun = rand.choice(nouns)
    suffix = random_suffix(6, rand=rand)
    return f"{prefix}_{adj}{noun}_{suffix}"


class AppearanceBase(SQLModel):
    color: str = Field(max_length=7, min_length=7)


class Appearance(AppearanceBase, table=True):
    id: int | None = Field(default=None, primary_key=True)


class AppearancePublic(AppearanceBase):
    pass


class AppearanceUpdate(AppearanceBase):
    color: str


class UserType(StrEnum):
    GUEST = "GUEST"
    PERMANENT = "PERMANENT"


class UserBase(SQLModel):
    username: str = Field(default="anonym", unique=True, max_length=100)
    user_type: UserType = Field(default=UserType.GUEST, sa_column=Integer)


class User(UserBase, table=True):
    id: uuid.UUID = Field(default_factory=uuid.uuid4, primary_key=True, index=True)
    password: Optional[str] = Field(default=None)  # use a hash in real applications
    appearance: Appearance = Relationship()
    appearance_id: int | None = Field(foreign_key="appearance.id", default=None)
    static_rooms: List["StaticRoom"] = Relationship(back_populates="owner")


class UserPublic(UserBase):
    appearance: AppearancePublic


class UserPrivate(UserPublic):
    id: uuid.UUID


class PermanentUserPrivate(UserPrivate):
    static_rooms: List["StaticRoom"]


class UserUpdate(UserBase):
    password: str
    new_password: Optional[str]
    appearance: Optional[AppearanceUpdate]
