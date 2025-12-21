"""
Basic tests to see if the most basic features work.
"""

import os

import fakeredis
import pytest
from dotenv import load_dotenv
from httpx import ASGITransport, AsyncClient
from sqlmodel import Session, SQLModel, StaticPool, create_engine

from app.dependencies import DatabaseContext, get_db_context
from app.main import app

load_dotenv()
debug_key = os.environ.get("DEV_API_KEY")


@pytest.fixture
def setup_test_db():
    engine = create_engine(
        "sqlite:///:memory:",
        connect_args={"check_same_thread": False},
        poolclass=StaticPool,
    )
    SQLModel.metadata.create_all(engine)

    async def _override_get_db_context():
        with Session(engine) as session:
            yield DatabaseContext(
                valkey=fakeredis.FakeAsyncValkey(), psql_session=session
            )

    app.dependency_overrides[get_db_context] = _override_get_db_context
    with Session(engine) as session:
        yield DatabaseContext(valkey=fakeredis.FakeAsyncValkey(), psql_session=session)


@pytest.mark.asyncio
async def test_example(setup_test_db: DatabaseContext):
    """just to show how a test looks"""
    async with AsyncClient(
        transport=ASGITransport(app=app),
        base_url="http://test",
        headers={"X-Api-Key": f"{debug_key}"},
    ) as _ac:
        pass
    assert True
