# \UsersApi

All URIs are relative to *https://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**users_get_me**](UsersApi.md#users_get_me) | **GET** /users/me | Get Me
[**users_login**](UsersApi.md#users_login) | **POST** /users/login | Login
[**users_online**](UsersApi.md#users_online) | **GET** /users/online | Online
[**users_register**](UsersApi.md#users_register) | **POST** /users/register | Register



## users_get_me

> models::UserPrivate users_get_me()
Get Me

Retrieve the currently authenticated user's information.  - **user**: The currently authenticated user dependency.  Returns: - The user information encapsulated in the UserPrivate model.

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::UserPrivate**](UserPrivate.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## users_login

> models::OnlineResponse users_login(login_data)
Login

Login a user using username and password.  - **login**: Contains the username and password for authentication.  Returns: - An access token and the user ID if login is successful.  Raises: - HTTPException: If credentials are invalid.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**login_data** | [**LoginData**](LoginData.md) |  | [required] |

### Return type

[**models::OnlineResponse**](OnlineResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## users_online

> models::OnlineResponse users_online(username)
Online

Get online user status or create a temporary user.  - **credentials**: An optional JWT token used for authentication. - **username**: An optional username parameter. If not provided, a temporary username will be generated.  Returns: - An access token and the user ID.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**username** | Option<**String**> |  |  |

### Return type

[**models::OnlineResponse**](OnlineResponse.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## users_register

> models::OnlineResponse users_register(login_data)
Register

Register a new user.  - **login**: Contains the username and password for registration. - **current_token**: An optional JWT token for authenticated registration.  Returns: - An access token and the user ID.  Raises: - HTTPException: If username is missing or user already exists.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**login_data** | [**LoginData**](LoginData.md) |  | [required] |

### Return type

[**models::OnlineResponse**](OnlineResponse.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

