import logging
import random
import string
from typing import List

from fastapi import APIRouter, Depends, HTTPException, status
from fastapi.responses import RedirectResponse
from sqlmodel import col, or_, select

from app.datamodel.message import *
from app.datamodel.user import *
from app.dependencies import (
    RESPONSES,
    DatabaseDependency,
    ErrorModel,
    PermanentUserDependency,
    get_current_user,
)

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

router = APIRouter(
    prefix="/rooms",
    tags=["rooms"],
    dependencies=[Depends(get_current_user)],
)


@router.get(
    "/",
    response_model=List[StaticRoomPublic],
    status_code=200,
    responses={**RESPONSES},
)
async def list_rooms(db: DatabaseDependency):
    """
    List all rooms.

    Args:
        db (DatabaseDependency): The database dependency for executing queries.

    Returns:
        List[StaticRoomPublic]: A list of public static room details.
    """
    logger.debug("Listing all rooms")
    stmt = select(StaticRoom)
    rooms = db.psql_session.exec(stmt).all()
    logger.debug(f"Found {len(rooms)} rooms")
    return rooms


@router.get(
    "/mine",
    response_model=List[StaticRoomPublic],
    status_code=200,
    responses={**RESPONSES},
)
async def get_my_rooms(user: PermanentUserDependency, db: DatabaseDependency):
    """
    Get all rooms owned by the current user.

    Args:
        user (PermanentUserDependency): The currently authenticated permanent user.
        db (DatabaseDependency): The database dependency for executing queries.

    Returns:
        List[StaticRoomPublic]: A list of rooms owned by the user.
    """
    logger.debug(f"Getting rooms for user: {user.username}")
    stmt = select(StaticRoom).where(StaticRoom.owner_id == user.id)
    rooms = db.psql_session.exec(stmt).all()
    logger.debug(f"User {user.username} owns {len(rooms)} rooms")
    return rooms


def generate_random_string(length: int = 8):
    """Generate a random string of fixed length."""
    letters = string.ascii_letters + string.digits
    return "".join(random.choice(letters) for _ in range(length))


@router.get("/room", tags=["experimental"])
async def random_room():
    random_string = generate_random_string()
    return random_string


# -------------------------------------------------------------
# CRUD for rooms
# -------------------------------------------------------------


@router.put(
    "/{room}",
    status_code=201,
    responses={
        **RESPONSES,
        409: {
            "model": ErrorModel,
            "description": "Retuned if the room already exists or the user has reached their room limit",
        },
    },
)
async def create_room(
    room: str,
    user: PermanentUserDependency,
    db: DatabaseDependency,
    room_data: CreateRoom,
):
    """
    Create a new room.

    Args:
        room (str): The name of the room to create.
        user (PermanentUserDependency): The currently authenticated permanent user.
        db (DatabaseDependency): The database dependency for executing queries.
        room_data (CreateRoom): Data for the new room including private level and invited users.

    Raises:
        HTTPException: If the room already exists.

    Returns:
        StaticRoomPublic: The newly created room's public details.
    """
    if len(user.static_rooms) + 1 > 10:
        raise HTTPException(
            status.HTTP_409_CONFLICT,
            detail="Room limit reached. You cannot create more rooms.",
        )
    logger.debug(f"Creating room: {room} by user: {user.username}")
    stmt = select(StaticRoom).where(StaticRoom.name == room)
    db_room = db.psql_session.exec(stmt).one_or_none()
    if db_room:
        logger.warning(f"Room {room} already exists")
        raise HTTPException(status.HTTP_409_CONFLICT, detail="Room already exists")

    users = []
    if room_data.invite:
        ids = [id for id in room_data.invite if isinstance(id, uuid.UUID)]
        names = [name for name in room_data.invite if isinstance(name, str)]
        stmt = select(User).where(
            or_(col(User.id).in_(ids), col(User.username).in_(names))
        )
        users: List[User] = list(db.psql_session.exec(stmt).all())

    new_room = StaticRoom(
        name=room,
        owner_id=user.id,
        level=room_data.private_level,
        users=users,
        key=room_data.key,
    )

    db.psql_session.add(new_room)
    db.psql_session.commit()
    db.psql_session.refresh(new_room)
    logger.debug(f"Room {room} created successfully")
    return StaticRoomPublic.model_validate(new_room)


@router.post(
    "/{room}",
    status_code=204,
    responses={
        **RESPONSES,
        404: {"model": ErrorModel, "description": "Retuned if the room cant be found"},
    },
)
async def update_room(
    user: PermanentUserDependency,
    db: DatabaseDependency,
    room: str,
    room_data: UpdateRoom,
):
    """
    Update an existing room.

    Args:
        user (PermanentUserDependency): The currently authenticated permanent user.
        db (DatabaseDependency): The database dependency for executing queries.
        room (str): The name of the room to update.
        room_data (UpdateRoom): Data for the updates, including private level, key, and invites.

    Raises:
        HTTPException: If the room does not exist or if unauthorized access is attempted.
    """
    logger.debug(f"Updating room: {room} by user: {user.username}")
    stmt = (
        select(StaticRoom)
        .where(StaticRoom.name == room)
        .where(StaticRoom.owner_id == user.id)
    )
    db_room = db.psql_session.exec(stmt).one_or_none()
    if db_room is None:
        logger.warning(
            f"Room {room} not found or access denied for user: {user.username}"
        )
        raise HTTPException(status.HTTP_404_NOT_FOUND, detail="Room doesn't exist")

    if room_data.private_level is not None:
        db_room.level = room_data.private_level
        logger.debug(f"Room {room} private level updated to {room_data.private_level}")
    if room_data.key is not None:
        db_room.key = room_data.key
        logger.debug(f"Room {room} key updated")
    if room_data.invite is not None:
        ids = [id for id in room_data.invite if isinstance(id, uuid.UUID)]
        names = [name for name in room_data.invite if isinstance(name, str)]
        stmt = select(User).where(
            or_(col(User.id).in_(ids), col(User.username).in_(names))
        )
        users: List[User] = list(db.psql_session.exec(stmt).all())
        db_room.users += users  # Add new users to the room
        logger.debug(f"Users added to room {room}: {[user.username for user in users]}")

    db.psql_session.add(db_room)
    db.psql_session.commit()
    db.psql_session.refresh(db_room)
    logger.debug(f"Room {room} updated successfully")


@router.delete(
    "/{room}",
    status_code=204,
    responses={
        **RESPONSES,
        404: {"model": ErrorModel, "description": "Retuned if the room cant be found"},
    },
)
async def delete_room(
    room: str,
    user: PermanentUserDependency,
    db: DatabaseDependency,
):
    """
    Delete an existing room.

    Args:
        room (str): The name of the room to delete.
        user (PermanentUserDependency): The currently authenticated permanent user.
        db (DatabaseDependency): The database dependency for executing queries.

    Raises:
        HTTPException: If the room does not exist or unauthorized access is attempted.
    """
    logger.debug(f"Attempting to delete room: {room} by user: {user.username}")
    stmt = (
        select(StaticRoom)
        .where(StaticRoom.name == room)
        .where(StaticRoom.owner_id == user.id)
    )
    db_room = db.psql_session.exec(stmt).one_or_none()
    if db_room is None:
        logger.warning(
            f"Room {room} not found or access denied for user: {user.username}"
        )
        raise HTTPException(status.HTTP_404_NOT_FOUND, detail="Room doesn't exist")

    db.psql_session.delete(db_room)
    db.psql_session.commit()
    logger.debug(f"Room {room} deleted successfully")


@router.get(
    "/{room}",
    response_model=list[MessagePublic],
    status_code=201,
    responses={
        **RESPONSES,
        404: {"model": ErrorModel, "description": "Retuned if the room cant be found"},
    },
)
async def get_room(
    room: str,
    user: PermanentUserDependency,
    db: DatabaseDependency,
):
    db_user = User.model_validate(user)
    db.psql_session.refresh(db_user)
    stmt = (
        select(StaticRoom)
        .where(StaticRoom.name == room)
        .where(or_(StaticRoom.owner == db_user, col(StaticRoom.users).in_(db_user)))
    )
    db_room = db.psql_session.exec(stmt).one_or_none()
    if not db_room:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND)
    stmt = (
        select(Message)
        .where(Message.room.name == db_room.name)
        .order_by(col(Message.send_at))
        .limit(10)
    )
    msgs = db.psql_session.exec(stmt).all()
    return [MessagePublic.model_validate(msg) for msg in msgs]
