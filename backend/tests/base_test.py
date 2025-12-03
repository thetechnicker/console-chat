"""
Basic tests to see if the most basic features work.
"""

import os

import fakeredis
import pytest
from app.main import app  # type:ignore
from app.main import DatabaseContext, get_db_context  # type:ignore
from dotenv import load_dotenv
from httpx import ASGITransport, AsyncClient
from sqlmodel import Session, SQLModel, StaticPool, create_engine

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
async def test_root(setup_test_db: DatabaseContext):
    async with AsyncClient(
        transport=ASGITransport(app=app),
        base_url="http://test",
        headers={"X-Api-Key": f"{debug_key}"},
    ) as ac:
        response = await ac.get("/")
    assert response.status_code == 200


@pytest.mark.asyncio
async def test_root_wrong_key(setup_test_db: DatabaseContext):
    async with AsyncClient(
        transport=ASGITransport(app=app),
        base_url="http://test",
        headers={"X-Api-Key": "WrongKey"},
    ) as ac:
        response = await ac.get("/")
    assert response.status_code == 401


@pytest.mark.asyncio
async def test_root_no_key(setup_test_db: DatabaseContext):
    async with AsyncClient(
        transport=ASGITransport(app=app),
        base_url="http://test",
    ) as ac:
        response = await ac.get("/")
    assert response.status_code == 403


@pytest.mark.asyncio
async def test_auth(setup_test_db: DatabaseContext):
    async with AsyncClient(
        transport=ASGITransport(app=app),
        base_url="http://test",
    ) as ac:
        response1 = await ac.post("/auth")
        response2 = await ac.post("/auth", json={"username": "test"})
        response3 = await ac.post(
            "/auth", json={"username": "test", "password": "test"}
        )
        response4 = await ac.post("/auth", json={"password": "test"})
    assert response1.status_code == 200, response1.text
    assert response2.status_code == 200, response2.text
    assert response3.status_code == 401, response3.text
    assert response4.status_code == 400, response3.text
