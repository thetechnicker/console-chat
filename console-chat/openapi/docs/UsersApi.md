# \UsersApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_me_users_me_get**](UsersApi.md#get_me_users_me_get) | **GET** /users/me | Get Me
[**login_users_login_post**](UsersApi.md#login_users_login_post) | **POST** /users/login | Login
[**online_users_online_get**](UsersApi.md#online_users_online_get) | **GET** /users/online | Online
[**register_users_register_post**](UsersApi.md#register_users_register_post) | **POST** /users/register | Register



## get_me_users_me_get

> models::UserPrivate get_me_users_me_get()
Get Me

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


## login_users_login_post

> models::OnlineResponse login_users_login_post(login_data)
Login

Login as permanent user

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


## online_users_online_get

> models::OnlineResponse online_users_online_get(username)
Online

Set Status to online/Create Auth Token for temporary user

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


## register_users_register_post

> models::OnlineResponse register_users_register_post(register_data)
Register

Register as permanent user

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**register_data** | [**RegisterData**](RegisterData.md) |  | [required] |

### Return type

[**models::OnlineResponse**](OnlineResponse.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

