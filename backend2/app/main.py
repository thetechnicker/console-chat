import atexit
import logging
import logging.config
import logging.handlers
import pathlib
import time
from typing import Any, cast

import asgi_correlation_id  # type:ignore
import yaml
from asgi_correlation_id import CorrelationIdMiddleware
from fastapi import FastAPI, Request
from fastapi.responses import HTMLResponse
from fastapi.routing import APIRoute

import app.logger  # type:ignore
from app.dependencies import lifespan
from app.routers import rooms, rooms_old, users, websockets


def setup_logging():
    config_file = pathlib.Path("logging_configs/5-queued-stderr-json-file.yaml")
    with open(config_file, "r") as f_in:
        config = yaml.safe_load(f_in)

    logging.config.dictConfig(config)
    queue_handler: logging.handlers.QueueHandler | None = cast(
        logging.handlers.QueueHandler | None, logging.getHandlerByName("queue_handler")
    )
    if queue_handler is not None:
        if queue_handler.listener is not None:
            queue_handler.listener.start()
            atexit.register(queue_handler.listener.stop)


setup_logging()


def custom_generate_unique_id(route: APIRoute):
    tag = route.tags[0] if len(route.tags) > 0 else "root"
    return f"{tag}-{route.name}"


app = FastAPI(
    title="Console Chat API",
    lifespan=lifespan,
    servers=[
        {"url": "https://localhost", "description": "local development environment"},
        {"url": "https://console-chat", "description": "with correct dns"},
    ],
    root_path="/api/v1",
    root_path_in_servers=False,
    generate_unique_id_function=custom_generate_unique_id,
)

LOG = logging.getLogger(__name__)
LOG.info("API is starting up")


@app.middleware("http")
async def log_requests(request: Request, call_next: Any):
    start_time = time.perf_counter()
    response = await call_next(request)
    response_time = time.perf_counter() - start_time
    LOG.info(
        f"{request.method} {request.url.path} {response.status_code} {response_time:.3f}s"
    )
    return response


app.add_middleware(CorrelationIdMiddleware)


@app.get("/")
def home():
    return HTMLResponse("Hello")


app.include_router(users.router)
app.include_router(rooms.router)
app.include_router(rooms_old.router)
app.include_router(websockets.router)
# app.include_router(admin.router)


# WARNING: this is for debug only
# @app.get("/valkey", response_model=list[str])
# async def valkey_get(db: DatabaseDependency):
#    keys = await db.valkey.keys()
#    return keys

if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app)
