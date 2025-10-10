from fastapi import FastAPI, Depends, HTTPException
from valkey import Valkey

app = FastAPI()
valkey = ValKey(secret="your_secret_key")


@app.post("/api/auth/login")
def login(username: str, password: str):
    # Authentication logic here
    return {"token": valkey.generate(username)}


@app.post("/api/messages")
def send_message(message: str, token: str = Depends(valkey.auth)):
    if not token.is_authenticated:
        raise HTTPException(status_code=401, detail="Unauthorized")
    # Logic to send message
    return {"message": "Message sent", "content": message}
