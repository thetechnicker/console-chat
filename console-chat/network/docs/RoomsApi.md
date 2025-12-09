# \RoomsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_room_rooms_room_put**](RoomsApi.md#create_room_rooms_room_put) | **PUT** /rooms/{room} | Create Room
[**delete_room_rooms_room_delete**](RoomsApi.md#delete_room_rooms_room_delete) | **DELETE** /rooms/{room} | Delete Room
[**get_my_rooms_rooms_mine_get**](RoomsApi.md#get_my_rooms_rooms_mine_get) | **GET** /rooms/mine | Get My Rooms
[**get_room_rooms_room_get**](RoomsApi.md#get_room_rooms_room_get) | **GET** /rooms/{room} | Get Room
[**list_rooms_rooms_get**](RoomsApi.md#list_rooms_rooms_get) | **GET** /rooms/ | List Rooms
[**update_room_rooms_room_post**](RoomsApi.md#update_room_rooms_room_post) | **POST** /rooms/{room} | Update Room



## create_room_rooms_room_put

> serde_json::Value create_room_rooms_room_put(room, create_room)
Create Room

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**room** | **String** |  | [required] |
**create_room** | [**CreateRoom**](CreateRoom.md) |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_room_rooms_room_delete

> serde_json::Value delete_room_rooms_room_delete(room)
Delete Room

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**room** | **String** |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_my_rooms_rooms_mine_get

> Vec<models::StaticRoomPublic> get_my_rooms_rooms_mine_get()
Get My Rooms

### Parameters

This endpoint does not need any parameter.

### Return type

[**Vec<models::StaticRoomPublic>**](StaticRoomPublic.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_room_rooms_room_get

> serde_json::Value get_room_rooms_room_get(room)
Get Room

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**room** | **String** |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_rooms_rooms_get

> Vec<models::StaticRoomPublic> list_rooms_rooms_get()
List Rooms

### Parameters

This endpoint does not need any parameter.

### Return type

[**Vec<models::StaticRoomPublic>**](StaticRoomPublic.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_room_rooms_room_post

> serde_json::Value update_room_rooms_room_post(room, update_room)
Update Room

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**room** | **String** |  | [required] |
**update_room** | [**UpdateRoom**](UpdateRoom.md) |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

