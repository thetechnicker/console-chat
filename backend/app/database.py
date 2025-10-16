import os
from typing import Optional
from sqlalchemy import create_engine, Column, Integer, String, Boolean, ForeignKey
from sqlalchemy.orm import declarative_base, relationship, sessionmaker


# ------------------------------------------------------------------------
#                                    DB User
# ------------------------------------------------------------------------


def set_connection_str(host: Optional[str] = None):
    user = os.getenv("POSTGRES_USER")
    password = os.getenv("POSTGRES_PASSWORD")
    if host is None:
        host = os.getenv("POSTGRES_HOST", "postgres")
    # host = "postgres"  # or actual host
    port = "5432"
    database = os.getenv("POSTGRES_DB")
    return f"postgresql://{user}:{password}@{host}:{port}/{database}"


Base = declarative_base()


class DBPublicUser(Base):
    __tablename__ = "public_user"

    id = Column(Integer, primary_key=True, autoincrement=True)  # unique identifier
    display_name = Column(String, nullable=False)
    # other public information fields here

    # You might add a one-to-one relationship back to BetterUser if desired
    better_user = relationship("DBUser", back_populates="public_data", uselist=False)


class DBUser(Base):
    __tablename__ = "users"

    id = Column(Integer, primary_key=True, autoincrement=True)  # unique identifier
    username = Column(String, unique=True, nullable=False)
    password_hash = Column(String, nullable=True)
    private = Column(Boolean, default=False)

    public_data_id = Column(Integer, ForeignKey("public_user.id"))
    public_data = relationship("DBPublicUser", back_populates="better_user")


def init_postgesql_connection():
    connection_str: str = set_connection_str()
    engine = create_engine(
        connection_str,
        pool_size=20,
        max_overflow=10,
        pool_recycle=3600,
        pool_timeout=30,
    )
    Base.metadata.create_all(engine)
    return sessionmaker(bind=engine)
