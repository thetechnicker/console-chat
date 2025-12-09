from fastapi import APIRouter, Depends, HTTPException, status
from sqlmodel import col, or_, select

from app.datamodel.message import *
from app.datamodel.user import *
from app.dependencies import (
    DatabaseDependency,
    PermanentUserDependency,
    UserDependency,
    get_current_user,
)

router = APIRouter(
    prefix="/rooms",
    tags=["rooms"],
    dependencies=[Depends(get_current_user)],
)


@router.get("/", response_model=List[StaticRoomPublic])
async def list_rooms(db: DatabaseDependency):
    stmt = select(StaticRoom)
    return db.psql_session.exec(stmt).all()


@router.get("/mine", response_model=List[StaticRoomPublic])
async def get_my_rooms(user: PermanentUserDependency, db: DatabaseDependency):
    stmt = select(StaticRoom).where(StaticRoom.owner_id == user.id)
    rooms = db.psql_session.exec(stmt).all()
    return rooms


@router.put("/{room}")
async def create_room(
    room: str,
    user: PermanentUserDependency,
    db: DatabaseDependency,
    room_data: CreateRoom,
):
    stmt = select(StaticRoom).where(StaticRoom.name == room)
    db_room = db.psql_session.exec(stmt).one_or_none()
    if db_room:
        raise HTTPException(status.HTTP_409_CONFLICT, detail="room already exists")
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
    return StaticRoomPublic.model_validate(new_room)


@router.post("/{room}")
async def update_room(
    user: PermanentUserDependency,
    db: DatabaseDependency,
    room: str,
    room_data: UpdateRoom,
):
    stmt = (
        select(StaticRoom)
        .where(StaticRoom.name == room)
        .where(StaticRoom.owner_id == user.id)
    )
    db_room = db.psql_session.exec(stmt).one_or_none()
    if db_room is None:
        raise HTTPException(status.HTTP_418_IM_A_TEAPOT, detail="room doesnt exist")
    if room_data.private_level is not None:
        db_room.level = room_data.private_level
    if room_data.key is not None:
        db_room.key = room_data.key
    if room_data.invite is not None:
        ids = [id for id in room_data.invite if isinstance(id, uuid.UUID)]
        names = [name for name in room_data.invite if isinstance(name, str)]
        stmt = select(User).where(
            or_(col(User.id).in_(ids), col(User.username).in_(names))
        )
        users: List[User] = list(db.psql_session.exec(stmt).all())
        db_room.users += users
    db.psql_session.add(db_room)
    db.psql_session.commit()
    db.psql_session.refresh(db_room)


@router.delete("/{room}")
async def delete_room(
    room: str,
    user: PermanentUserDependency,
    db: DatabaseDependency,
):
    stmt = (
        select(StaticRoom)
        .where(StaticRoom.name == room)
        .where(StaticRoom.owner_id == user.id)
    )
    db_room = db.psql_session.exec(stmt).one_or_none()
    if db_room is None:
        raise HTTPException(status.HTTP_418_IM_A_TEAPOT, detail="room doesnt exist")
    db.psql_session.delete(db_room)


@router.get("/{room}")
async def get_room(
    room: str,
    user: UserDependency,
    db: DatabaseDependency,
):
    stmt = select(StaticRoom).where(StaticRoom.name == room)
    db_room = db.psql_session.exec(stmt).one_or_none()
    if db_room:
        if db_room.level == RoomLevel.FREE:
            return True
        if user.user_type == UserType.GUEST:
            return False
        full_user = User.model_validate(user)
        db.psql_session.refresh(full_user)
        if full_user in db_room.users:
            return True
    else:
        return True
