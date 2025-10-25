"""
Fixtures for backend tests.

Adjust the import paths at the top to match your project's layout.
This file provides:
- a TestClient (httpx.AsyncClient) for the FastAPI app
- an in-memory SQLite session override (if your app uses SQLAlchemy sync sessions)
- a FakeValkey that mimics the minimal valkey/pool API used by the app
- an override for the app's get_context dependency so endpoints talk to the fake valkey and test DB

Notes:
- You will very likely need to change the import paths for `app`, `get_context`, and `Base`.
- If your app uses async SQLAlchemy (AsyncSession) you can adapt the session creation similarly.
"""

import asyncio
from types import SimpleNamespace

import pytest
import httpx
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

# Pydantic / FastAPI imports
try:
    # try common project layout
    from app.main import app, get_context
    from app.database import Base  # Base metadata for models
except Exception:
    try:
        # alternative layout used in some examples
        from backend.app.main import app, get_context
        from backend.app.database import Base
    except Exception as e:
        raise ImportError(
            "Couldn't import app, get_context or Base. "
            "Adjust the import paths in tests/conftest.py to match your project layout."
        ) from e


# --- Fake Valkey implementation used by endpoints in tests ---
class FakePubSub:
    def __init__(self, published_messages):
        self.published_messages = published_messages
        self._subscribed = set()
        self._messages = []
        # put a default system JOIN message template if desired:
        self._messages.append(
            b'{"type":"SYSTEM","text":"joined","user":{"display_name":"tester","private":false}}'
        )

    async def __aenter__(self):
        return self

    async def __aexit__(self, exc_type, exc, tb):
        return False

    async def subscribe(self, channel):
        self._subscribed.add(channel)

    async def get_message(self, ignore_subscribe_messages=True, timeout=None):
        # Pop from the queue if available, else return None to indicate timeout
        if self._messages:
            return {"data": self._messages.pop(0)}
        # simulate timeout / end of stream
        await asyncio.sleep(0)
        return None

    # helper to inject messages for streaming tests
    def inject(self, raw_bytes):
        self._messages.append(raw_bytes)


class FakeValkey:
    def __init__(self):
        self.published = []  # list of tuples (channel, payload_bytes)
        self.pubsub_instance = FakePubSub(self.published)

    async def ping(self):
        return True

    async def publish(self, channel: str, message: bytes):
        # emulate valkey returning something
        self.published.append((channel, message))
        return {"ok": True}

    async def pubsub(self):
        return self.pubsub_instance


# --- SQLAlchemy in-memory engine and sessionmaker ---
# Using synchronous engine/session for simplicity. Adapt if your app uses async DB.
_TEST_ENGINE = create_engine(
    "sqlite:///:memory:", connect_args={"check_same_thread": False}
)
_TestSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=_TEST_ENGINE)


@pytest.fixture(scope="session", autouse=True)
def init_models():
    """
    Create tables once per pytest session.
    If your project uses alembic or dynamic model registration,
    adapt this to import your model Base metadata and create tables.
    """
    Base.metadata.create_all(bind=_TEST_ENGINE)
    yield
    Base.metadata.drop_all(bind=_TEST_ENGINE)


@pytest.fixture()
def db_session():
    """
    Provide a clean DB session for each test.
    Endpoint code that depends on a sessionmaker should be overridden to use this session.
    """
    session = _TestSessionLocal()
    try:
        yield session
    finally:
        session.close()


@pytest.fixture()
async def fake_valkey():
    return FakeValkey()


@pytest.fixture()
async def override_get_context(monkeypatch, db_session, fake_valkey):
    """
    Override the FastAPI get_context dependency to yield a test context
    with p == db_session and valkey==fake_valkey.

    This assumes your app defines a dependency function `get_context` which is an async generator.
    Adjust as needed.
    """

    async def _override_get_context():
        yield SimpleNamespace(p=db_session, valkey=fake_valkey)

    # apply override for the running app
    app.dependency_overrides[get_context] = _override_get_context
    yield
    app.dependency_overrides.pop(get_context, None)


@pytest.fixture()
async def async_client(override_get_context):
    """
    Async test client bound to the FastAPI app. Use httpx.AsyncClient with the app.
    """
    async with httpx.AsyncClient(app=app, base_url="http://testserver") as client:
        yield client
