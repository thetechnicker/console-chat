import uuid
from typing import Annotated

from fastapi import APIRouter, Body, Depends, HTTPException, status
from fastapi.exceptions import HTTPException
from fastapi.security import OAuth2PasswordRequestForm
from sqlmodel import select

from app.datamodel.user import (
    Appearance,
    AppearancePublic,
    User,
    UserPrivate,
    UserPublic,
    UserType,
)
from app.dependencies import (
    TOKEN_TTL,
    ApiKeyAuth,
    DatabaseContext,
    DatabaseDependencie,
    LoginData,
    OnlineResponce,
    OptionalTokenDependencie,
    RegisterData,
    create_access_token,
    deterministic_color_from_string,
    get_db_context,
    get_user_from_token,
    secure_hash_argon2,
    verify_password,
)

router = APIRouter(
    prefix="/users",
    tags=["users"],
)


@router.get("/", response_model=list[UserPublic])
async def users(
    db_context: DatabaseDependencie,
    _: ApiKeyAuth,
):
    stmt = select(User)
    result = db_context.psql_session.exec(stmt)
    return [UserPublic.model_validate(m) for m in result.all()]


@router.get("/online", response_model=OnlineResponce)
async def online(
    db_context: DatabaseDependencie,
    credentials: OptionalTokenDependencie,
):
    # Handle Bearer Token Authentication
    if credentials:
        user = await get_user_from_token(credentials.credentials, db_context)
        if user:
            token = create_access_token(user, TOKEN_TTL)
            return OnlineResponce(token=token, user=user.id)

    id = uuid.uuid4()
    user = UserPrivate(
        id=id,
        appearance=AppearancePublic(color=deterministic_color_from_string(str(id))),
    )
    user_complete = User.model_validate(user)
    db_context.valkey.set(str(user_complete.id), user_complete.model_dump_json())
    token = create_access_token(user, TOKEN_TTL)
    return OnlineResponce(token=token, user=user_complete.id)


@router.post("/login", response_model=OnlineResponce)
async def login(
    login: Annotated[LoginData, Body()],
    db_context: DatabaseContext = Depends(get_db_context),
):
    # Handle Username and Password Authentication
    stmt = select(User).where(User.username == login.username)
    user = db_context.psql_session.exec(stmt).one_or_none()
    if (
        user
        and user.password
        and verify_password(user.password, login.username, login.password)
    ):
        token = create_access_token(user, TOKEN_TTL)

        return OnlineResponce(token=token, user=user.id)
    else:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid credentials"
        )


@router.post("/register", response_model=OnlineResponce)
async def register(
    login: Annotated[RegisterData, Body()],
    db_context: DatabaseDependencie,
    current_token: OptionalTokenDependencie,
):
    if login.username is None and (current_token is None):
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="you cant register without a username",
        )
    stmt = select(User).where(User.username == login.username)
    existing_user = db_context.psql_session.exec(stmt).one_or_none()
    if existing_user:
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT, detail="User already exists"
        )
    else:
        if current_token:
            user_from_token = await get_user_from_token(
                current_token.credentials, db_context
            )
            print(user_from_token)
        elif login.username:
            password = secure_hash_argon2(login.username, login.password)
            appearance = Appearance(
                color=deterministic_color_from_string(login.username)
            )
            db_context.psql_session.add(appearance)
            db_context.psql_session.commit()
            new_user = User(
                username=login.username,
                user_type=UserType.PERMANENT,
                password=password,
                appearance=appearance,
            )
            db_context.psql_session.add(new_user)
            db_context.psql_session.commit()
            db_context.psql_session.refresh(new_user)
            token = create_access_token(new_user, TOKEN_TTL)
            return OnlineResponce(token=token, user=new_user.id)


@router.post("/token")
async def login_oauth(
    form_data: Annotated[OAuth2PasswordRequestForm, Depends()],
    db_context: DatabaseDependencie,
):
    stmt = select(User).where(User.username == form_data.username)
    user = db_context.psql_session.exec(stmt).one_or_none()
    if (
        user
        and user.password
        and verify_password(user.password, form_data.username, form_data.password)
    ):
        token = create_access_token(user, TOKEN_TTL)

        return OnlineResponce(token=token, user=user.id)
    else:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid credentials"
        )
