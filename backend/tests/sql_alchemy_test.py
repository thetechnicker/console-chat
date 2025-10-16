import os

from dotenv import load_dotenv
from sqlalchemy import Column, Integer, String, create_engine
from sqlalchemy.orm import declarative_base, sessionmaker

load_dotenv()

user = os.getenv("POSTGRES_USER")
password = os.getenv("POSTGRES_PASSWORD")
host = "localhost"  # or actual host
port = "5432"
database = "DEBUG"

connection_str = f"postgresql://{user}:{password}@{host}:{port}/{database}"
# print(connection_str)
# exit()

engine = create_engine(connection_str)


# Base class for models
Base = declarative_base()


# Define a model class mapped to a table
class User(Base):
    __tablename__ = "users"

    id = Column(Integer, primary_key=True)
    name = Column(String)
    email = Column(String)


# Create tables in database
Base.metadata.create_all(engine)

# Create a session
Session = sessionmaker(bind=engine)
session = Session()

# Add a new user
new_user = User(name="Alice", email="alice@example.com")
session.add(new_user)
session.commit()
