import os
from typing import Optional, Any

from sqlmodel import Field, Relationship, SQLModel, create_engine


def set_connection_str(host: Optional[str] = None):
    user = os.getenv("POSTGRES_USER")
    password = os.getenv("POSTGRES_PASSWORD")
    if host is None:
        host = os.getenv("POSTGRES_HOST", "postgres")
    # host = "postgres"  # or actual host
    port = "5432"
    database = os.getenv("POSTGRES_DB")
    return f"postgresql://{user}:{password}@{host}:{port}/{database}"


class DBPublicUser(SQLModel, table=True):
    __tablename__ = "public_user"  # type: ignore
    id: Optional[int] = Field(default=None, primary_key=True)
    display_name: str
    color: Optional[str]
    better_user: Optional["DBUser"] = Relationship(back_populates="public_data")


class DBUser(SQLModel, table=True):
    __tablename__ = "users"  # type: ignore
    id: Optional[int] = Field(default=None, primary_key=True)
    username: str = Field(index=True)
    password_hash: Optional[str] = None
    private: bool = Field(default=False)
    public_data_id: Optional[int] = Field(default=None, foreign_key="public_user.id")
    public_data: DBPublicUser = Relationship(back_populates="better_user")

    def model_dump(self, **kwargs: Any):
        # Serialize full internal version by default (e.g., database)
        data = super().model_dump(**kwargs)
        data["public_data"] = self.public_data.model_dump()

        data["password_hash"] = None

        return data


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
