from dotenv import load_dotenv
import os
import pytest
from sqlmodel import Session, SQLModel, create_engine, StaticPool
from httpx import ASGITransport, AsyncClient
import fakeredis

from app.main import DatabaseContext, get_db_context  # type:ignore
from app.main import app  # type:ignore


load_dotenv()
debug_key = os.environ.get("DEV_API_KEY")


@pytest.mark.asyncio
async def test_root():
    async with AsyncClient(
        transport=ASGITransport(app=app),
        base_url="http://test",
        headers={"X-Api-Key": f"{debug_key}"},
    ) as ac:
        response = await ac.get("/")
    assert response.status_code == 200


@pytest.mark.asyncio
async def test_root_wrong_key():
    async with AsyncClient(
        transport=ASGITransport(app=app),
        base_url="http://test",
        headers={"X-Api-Key": "WrongKey"},
    ) as ac:
        response = await ac.get("/")
    assert response.status_code == 401


@pytest.mark.asyncio
async def test_root_no_key():
    async with AsyncClient(
        transport=ASGITransport(app=app),
        base_url="http://test",
    ) as ac:
        response = await ac.get("/")
    assert response.status_code == 403


@pytest.mark.asyncio
async def test_auth():
    engine = create_engine(
        "sqlite:///:memory:",
        connect_args={"check_same_thread": False},
        poolclass=StaticPool,
    )
    SQLModel.metadata.create_all(engine)

    async def overwrite_get_db_content():
        with Session(engine) as session:
            yield DatabaseContext(
                valkey=fakeredis.FakeAsyncValkey(), psql_session=session
            )

    app.dependency_overrides[get_db_context] = overwrite_get_db_content

    async with AsyncClient(
        transport=ASGITransport(app=app),
        base_url="http://test",
    ) as ac:
        response1 = await ac.post("/auth")
        response2 = await ac.post("/auth", json={"username": "test"})
        response3 = await ac.post(
            "/auth", json={"username": "test", "password": "test"}
        )
    assert response1.status_code == 200, response1.text
    assert response2.status_code == 200, response2.text
    assert response3.status_code == 401, response3.text
