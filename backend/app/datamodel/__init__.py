import os
from typing import Optional

from sqlmodel import SQLModel, create_engine

from .message import *
from .user import *


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
    return engine


User.model_rebuild()
PermanentUserPrivate.model_rebuild()
