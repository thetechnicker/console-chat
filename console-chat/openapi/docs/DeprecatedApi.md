# \DeprecatedApi

All URIs are relative to *https://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**deprecated_listen**](DeprecatedApi.md#deprecated_listen) | **GET** /room/{room} | Listen
[**deprecated_send**](DeprecatedApi.md#deprecated_send) | **POST** /room/{room} | Send



## deprecated_listen

> serde_json::Value deprecated_listen(room, listen_seconds)
Listen

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**room** | **String** |  | [required] |
**listen_seconds** | Option<**i32**> | How long to listen in seconds |  |[default to 30]

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## deprecated_send

> serde_json::Value deprecated_send(room, message_send)
Send

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**room** | **String** |  | [required] |
**message_send** | [**MessageSend**](MessageSend.md) |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

