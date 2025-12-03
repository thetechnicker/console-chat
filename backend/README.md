# ConsoleChat Backend

This directory contains the backend logic and services for **ConsoleChat**.

## Project Structure

- **app/** – Core backend application code  
- **tests/** – Unit and integration tests for the backend  
- **experiments/** – Experimental scripts and prototypes for exploring new functionality or understanding existing components

## Python Requirements

There are three requirement files:
- `requirements.txt`: Basic requirements for this application.
- requirements-dev.txt: Additional dependencies for development and testing.
- requirements-frozen.txt: Combines the other two requirement files and adds version restrictions to ensure consistency.

## Setup Instructions

Follow these steps to run the backend locally with Docker.

### 1. Create a `.env` File

Before starting Docker, create a `.env` file in the backend directory with the following variables:

```bash
# Security
SECRET=<secure-random-string>
DEV_API_KEY=<secure-random-string>

# Database
POSTGRES_USER=consolechat_user
POSTGRES_PASSWORD=consolechat_pass
POSTGRES_DB=consolechat_db
```

**Notes:**  
- Both `SECRET` and `DEV_API_KEY` should be securely generated random strings.  
  Example using OpenSSL:
  ```bash
  openssl rand -hex 32
  ```
- The `.env` file is required for the Docker setup, but `DEV_API_KEY` can be safely left out to disable developer-only routes.

### 2. Generate SSL Certificates (for nginx)

For local testing, generate a self-signed certificate:

```bash
openssl req -x509 -nodes -days 365 \
-newkey rsa:2048 \
-keyout ./nginx/certs/privkey.pem \
-out ./nginx/certs/fullchain.pem \
-subj "/CN=localhost"
```

### 3. Start Docker Compose

Build and start all backend containers:

```bash
docker compose up --build -d
```

This launches the backend services and nginx reverse proxy using your `.env` configuration and local SSL certificates.
