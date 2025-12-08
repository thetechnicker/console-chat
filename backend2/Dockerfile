FROM python:3.12-slim


WORKDIR /code


COPY ./requirements.txt /code/requirements.txt


RUN pip install --no-cache-dir --upgrade -r /code/requirements.txt


COPY ./logging_configs /code/logging_configs
COPY ./app /code/app

ARG PORT=8000
ENV PORT=${PORT}
EXPOSE ${PORT}


CMD ["fastapi", "run", "app/main.py", "--proxy-headers"]
