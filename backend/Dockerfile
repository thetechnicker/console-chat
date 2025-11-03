# Use the official Python image from the Docker Hub
FROM python:3.12-slim

# User
RUN useradd -ms /bin/sh -u 1001 app
USER app

# Set the working directory
WORKDIR /app

# Copy the requirements file and install dependencies
COPY requirements.txt ./
RUN pip install --upgrade pip
RUN pip install --no-cache-dir -r requirements.txt
#RUN pip install netifaces

# Copy the rest of the application code
COPY --chown=app:app . /app

# Expose the port on which FastAPI will run
#EXPOSE 8000

# Command to run the FastAPI application
CMD ["python", "-m", "uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8000"]
