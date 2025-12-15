import uuid
from typing import Annotated

from fastapi import APIRouter, Body, Depends, HTTPException, Query, status
from fastapi.exceptions import HTTPException
from sqlmodel import select

from app.datamodel.user import (
    Appearance,
    AppearancePublic,
    User,
    UserPrivate,
    UserType,
    generate_temp_username,
)
from app.dependencies import (
    TOKEN_TTL,
    DatabaseContext,
    DatabaseDependency,
    LoginData,
    OnlineResponse,
    OptionalTokenDependency,
    RegisterData,
    UserDependency,
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


@router.get(
    "/online",
    response_model=OnlineResponse,
)
async def online(
    db_context: DatabaseDependency,
    credentials: OptionalTokenDependency,
    username: Annotated[str | None, Query()] = None,
):
    """
    Get online user status or create a temporary user.

    - **credentials**: An optional JWT token used for authentication.
    - **username**: An optional username parameter. If not provided, a temporary username will be generated.

    Returns:
    - An access token and the user ID.
    """
    # Handle Bearer Token Authentication
    username = None
    if credentials:
        user = await get_user_from_token(credentials.credentials, db_context)
        if user.user_type == UserType.GUEST:
            await db_context.valkey.expire(str(user.id), TOKEN_TTL)
        token = create_access_token(user, TOKEN_TTL)
        return OnlineResponse(token=token, user=user.id)

    id = uuid.uuid4()
    if username is None:
        username = generate_temp_username(id)
    user = UserPrivate(
        id=id,
        username=username,
        appearance=AppearancePublic(color=deterministic_color_from_string(str(id))),
    )

    await db_context.valkey.set(str(user.id), user.model_dump_json(), ex=TOKEN_TTL)
    token = create_access_token(user, TOKEN_TTL)
    return OnlineResponse(token=token, user=user.id)


@router.post("/login", response_model=OnlineResponse)
async def login(
    login: Annotated[LoginData, Body()],
    db_context: DatabaseContext = Depends(get_db_context),
):
    """
    Login a user using username and password.

    - **login**: Contains the username and password for authentication.

    Returns:
    - An access token and the user ID if login is successful.

    Raises:
    - HTTPException: If credentials are invalid.
    """
    # Handle Username and Password Authentication
    stmt = select(User).where(User.username == login.username)
    user = db_context.psql_session.exec(stmt).one_or_none()

    if (
        user
        and user.password
        and verify_password(user.password, login.username, login.password)
    ):
        token = create_access_token(user, TOKEN_TTL)
        return OnlineResponse(token=token, user=user.id)
    else:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid credentials"
        )


@router.post(
    "/register",
    response_model=OnlineResponse,
    status_code=201,
)
async def register(
    login: Annotated[RegisterData, Body()],
    db_context: DatabaseDependency,
    current_token: OptionalTokenDependency,
):
    """
    Register a new user.

    - **login**: Contains the username and password for registration.
    - **current_token**: An optional JWT token for authenticated registration.

    Returns:
    - An access token and the user ID.

    Raises:
    - HTTPException: If username is missing or user already exists.
    """
    if login.username is None and (current_token is None):
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="You can't register without a username",
        )

    stmt = select(User).where(User.username == login.username)
    existing_user = db_context.psql_session.exec(stmt).one_or_none()

    if existing_user:
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT, detail="User already exists"
        )

    if current_token:
        user_from_token = await get_user_from_token(
            current_token.credentials, db_context
        )
        print(user_from_token)
    elif login.username:
        password = secure_hash_argon2(login.username, login.password)
        appearance = Appearance(color=deterministic_color_from_string(login.username))
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

        return OnlineResponse(token=token, user=new_user.id)


@router.get("/me", response_model=UserPrivate)
async def get_me(user: UserDependency):
    """
    Retrieve the currently authenticated user's information.

    - **user**: The currently authenticated user dependency.

    Returns:
    - The user information encapsulated in the UserPrivate model.
    """
    return UserPrivate.model_validate(user)
