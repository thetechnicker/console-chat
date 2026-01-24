"""
Basic tests to see if the most basic features work.
"""

import os
from datetime import datetime

import fakeredis
import pytest
from dotenv import load_dotenv
from httpx import ASGITransport, AsyncClient
from sqlmodel import Session, SQLModel, StaticPool, create_engine

from app.datamodel.message import MessagePublic, MessageSend, MessageType, Plaintext
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


# TODO: FIX

# @pytest.mark.asyncio
# async def test_room(setup_test_db: DatabaseContext):
#    """just to show how a test looks"""
#    async with AsyncClient(
#        transport=ASGITransport(app=app),
#        base_url="http://test",
#        headers={"X-Api-Key": f"{debug_key}"},
#    ) as ac:
#        online = await ac.get("users/online")
#
#        valid_token = online.json()["token"]["token"]
#
#        response = await ac.get(
#            "room/abc",
#            headers={
#                "Authorization": f"Bearer {valid_token}",
#                "Accept": "text/event-stream",
#            },
#        )
#
#        message = MessageSend(
#            type=MessageType.PLAINTEXT,
#            content=Plaintext(content="TEST"),
#            send_at=datetime.now(),
#        )
#        assert response.status_code == 200
#        assert response.headers["Content-Type"] == "text/event-stream"
#
#        async for chunk in response.aiter_text():
#            message_got = MessageSend.model_validate_json(chunk)
#            if message_got.type == MessageType.PLAINTEXT:
#                assert message == message_got
#                break
#        else:
#            assert False
