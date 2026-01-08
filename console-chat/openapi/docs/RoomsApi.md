# \RoomsApi

All URIs are relative to *https://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**rooms_create_room**](RoomsApi.md#rooms_create_room) | **PUT** /rooms/{room} | Create Room
[**rooms_delete_room**](RoomsApi.md#rooms_delete_room) | **DELETE** /rooms/{room} | Delete Room
[**rooms_get_my_rooms**](RoomsApi.md#rooms_get_my_rooms) | **GET** /rooms/mine | Get My Rooms
[**rooms_get_room**](RoomsApi.md#rooms_get_room) | **GET** /rooms/{room} | Get Room
[**rooms_list_rooms**](RoomsApi.md#rooms_list_rooms) | **GET** /rooms/ | List Rooms
[**rooms_listen**](RoomsApi.md#rooms_listen) | **GET** /room/{room} | Listen
[**rooms_listen_static**](RoomsApi.md#rooms_listen_static) | **GET** /room/static/{room} | Listen Static
[**rooms_random_room**](RoomsApi.md#rooms_random_room) | **GET** /rooms/room | Random Room
[**rooms_send**](RoomsApi.md#rooms_send) | **POST** /room/{room} | Send
[**rooms_send_static**](RoomsApi.md#rooms_send_static) | **POST** /room/static/{room} | Send Static
[**rooms_update_room**](RoomsApi.md#rooms_update_room) | **POST** /rooms/{room} | Update Room



## rooms_create_room

> serde_json::Value rooms_create_room(room, create_room)
Create Room

Create a new room.  Args:     room (str): The name of the room to create.     user (PermanentUserDependency): The currently authenticated permanent user.     db (DatabaseDependency): The database dependency for executing queries.     room_data (CreateRoom): Data for the new room including private level and invited users.  Raises:     HTTPException: If the room already exists.  Returns:     StaticRoomPublic: The newly created room's public details.

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


## rooms_delete_room

> rooms_delete_room(room)
Delete Room

Delete an existing room.  Args:     room (str): The name of the room to delete.     user (PermanentUserDependency): The currently authenticated permanent user.     db (DatabaseDependency): The database dependency for executing queries.  Raises:     HTTPException: If the room does not exist or unauthorized access is attempted.

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
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## rooms_get_my_rooms

> Vec<models::StaticRoomPublic> rooms_get_my_rooms()
Get My Rooms

Get all rooms owned by the current user.  Args:     user (PermanentUserDependency): The currently authenticated permanent user.     db (DatabaseDependency): The database dependency for executing queries.  Returns:     List[StaticRoomPublic]: A list of rooms owned by the user.

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


## rooms_get_room

> Vec<models::MessagePublic> rooms_get_room(room)
Get Room

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**room** | **String** |  | [required] |

### Return type

[**Vec<models::MessagePublic>**](MessagePublic.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## rooms_list_rooms

> Vec<models::StaticRoomPublic> rooms_list_rooms()
List Rooms

List all rooms.  Args:     db (DatabaseDependency): The database dependency for executing queries.  Returns:     List[StaticRoomPublic]: A list of public static room details.

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


## rooms_random_room

> String rooms_random_room()
Random Room

### Parameters

This endpoint does not need any parameter.

### Return type

**String**

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## rooms_send

> serde_json::Value rooms_send(room, message_send)
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


## rooms_send_static

> serde_json::Value rooms_send_static(room, message_send)
Send Static

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


## rooms_update_room

> rooms_update_room(room, update_room)
Update Room

Update an existing room.  Args:     user (PermanentUserDependency): The currently authenticated permanent user.     db (DatabaseDependency): The database dependency for executing queries.     room (str): The name of the room to update.     room_data (UpdateRoom): Data for the updates, including private level, key, and invites.  Raises:     HTTPException: If the room does not exist or if unauthorized access is attempted.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**room** | **String** |  | [required] |
**update_room** | [**UpdateRoom**](UpdateRoom.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[HTTPBearer](../README.md#HTTPBearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

