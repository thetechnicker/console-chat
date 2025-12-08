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

import app.logger  # type:ignore
from app.dependencies import lifespan
from app.routers import rooms, users, websockets


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


app = FastAPI(
    title="Console Chat API",
    lifespan=lifespan,
    root_path="/api/v1",
    root_path_in_servers=False,
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


app.include_router(users.router)
app.include_router(rooms.router)
app.include_router(websockets.router)
