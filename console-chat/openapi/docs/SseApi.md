# \SseApi

All URIs are relative to *https://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**rooms_listen**](SseApi.md#rooms_listen) | **GET** /room/{room} | Listen
[**rooms_listen_static**](SseApi.md#rooms_listen_static) | **GET** /room/static/{room} | Listen Static
[**rooms_send**](SseApi.md#rooms_send) | **POST** /room/{room} | Send
[**rooms_send_static**](SseApi.md#rooms_send_static) | **POST** /room/static/{room} | Send Static



## rooms_listen

> rooms_listen(room)
Listen

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**room** | **String** |  | [required] |

### Return type

 (empty response body)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: text/event-stream, application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## rooms_listen_static

> rooms_listen_static(room)
Listen Static

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**room** | **String** |  | [required] |

### Return type

 (empty response body)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: text/event-stream, application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## rooms_send

> models::MessagePublic rooms_send(room, message_send)
Send

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**room** | **String** |  | [required] |
**message_send** | [**MessageSend**](MessageSend.md) |  | [required] |

### Return type

[**models::MessagePublic**](MessagePublic.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## rooms_send_static

> models::MessagePublic rooms_send_static(room, message_send)
Send Static

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**room** | **String** |  | [required] |
**message_send** | [**MessageSend**](MessageSend.md) |  | [required] |

### Return type

[**models::MessagePublic**](MessagePublic.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

