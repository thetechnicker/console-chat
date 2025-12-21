from datetime import datetime, timedelta, timezone
from typing import Annotated, Any

import jwt
from fastapi import APIRouter, Depends, HTTPException, Security, status
from fastapi.security import OAuth2PasswordBearer, OAuth2PasswordRequestForm
from pydantic import BaseModel

from app.datamodel.user import *
from app.dependencies import (
    ALGORITHM,
    PRIVATE_JWT_KEY,  # PUBLIC_JWT_KEY,
    TOKEN_TTL,
    DatabaseDependency,
    OptionalTokenDependency,
    get_current_permanent_user,
    get_from_login,
)


class Token(BaseModel):
    access_token: str
    token_type: str


class TokenData(BaseModel):
    username: str | None = None
    scopes: list[str] = []


oauth2_scheme = OAuth2PasswordBearer(
    tokenUrl="admin/token",
    scopes={
        "read:user": "Grants permission to read user information.",
        "write:user": "Grants permission to modify user information.",
        "delete:user": "Grants permission to delete user accounts.",
        "read:rooms": "Grants permission to read room details.",
        "write:rooms": "Grants permission to create or update rooms.",
        "delete:rooms": "Grants permission to remove room entries.",
    },
)

router = APIRouter(
    prefix="/admin",
    tags=["admin"],
    dependencies=[Security(oauth2_scheme)],
)


def create_access_token(data: dict[str, Any], expires_delta: timedelta | None = None):
    to_encode = data.copy()
    if expires_delta:
        expire = datetime.now(timezone.utc) + expires_delta
    else:
        expire = datetime.now(timezone.utc) + timedelta(minutes=15)
    to_encode.update({"exp": expire})
    encoded_jwt = jwt.encode(to_encode, PRIVATE_JWT_KEY, algorithm=ALGORITHM)
    return encoded_jwt


@router.post("/token")
async def login_admin(
    form_data: Annotated[Optional[OAuth2PasswordRequestForm], Depends()],
    user_token: OptionalTokenDependency,
    db: DatabaseDependency,
):
    if user_token:
        user = await get_current_permanent_user(user_token, db)
        scopes = []
    elif form_data:
        user = get_from_login(form_data.username, form_data.password, db)
        if not user:
            raise HTTPException(
                status_code=400, detail="Incorrect username or password"
            )
        scopes = form_data.scopes
    else:
        raise HTTPException(status.HTTP_401_UNAUTHORIZED)

    data = {"sub": str(user.id), "scope": " ".join(scopes)}
    access_token_expires = timedelta(minutes=TOKEN_TTL)
    access_token = create_access_token(data, access_token_expires)
    return Token(access_token=access_token, token_type="bearer")
