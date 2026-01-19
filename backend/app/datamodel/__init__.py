import logging
import os
from typing import Optional

from sqlalchemy import Engine
from sqlmodel import SQLModel, create_engine

from .message import *
from .user import *

logger = logging.getLogger(__name__)


def set_connection_str(host: Optional[str] = None):
    user = os.getenv("POSTGRES_USER")
    password = os.getenv("POSTGRES_PASSWORD")
    if host is None:
        host = os.getenv("POSTGRES_HOST", "postgres")
    port = "5432"
    database = os.getenv("POSTGRES_DB")
    return f"postgresql://{user}:{password}@{host}:{port}/{database}"


engine = None


def init_postgesql_connection():
    global engine
    connection_str: str = set_connection_str()
    engine = create_engine(
        connection_str,
        pool_size=20,
        max_overflow=10,
        pool_recycle=3600,
        pool_timeout=30,
    )
    SQLModel.metadata.create_all(engine)
    create_or_get_system(engine)
    return engine


def create_or_get_system(engine: Engine):
    global SYSTEM_USER
    from sqlmodel import Session, select

    with Session(engine) as session:
        stmt = select(User).where(User.username == "SYSTEM")
        user = session.exec(stmt).one_or_none()
        if user is None:
            user = User(
                username="SYSTEM",
                user_type=UserType.SYSTEM,
                appearance=Appearance(color="#333333"),
            )
            session.add(user)
            session.commit()
            session.refresh(user)
        SYSTEM_USER = User.model_validate(user)  # type: ignore
        logger.debug(f"System user: {user}")


User.model_rebuild()
PermanentUserPrivate.model_rebuild()

SYSTEM_USER: User = User(
    username="SYSTEM",
    user_type=UserType.SYSTEM,
    appearance=Appearance(color="#333333"),
)
