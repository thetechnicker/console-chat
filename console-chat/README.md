# console-chat-3


## Api Interaction

### Init

```mermaid
sequenceDiagram
box Client
    participant UI
    participant Client
end
box Server 
    participant RestApi
    participant DB
end

Client ->> RestApi: Request Token for anonym session
RestApi ->> Client: Token for anonym User

```

### Login

```mermaid
sequenceDiagram
box Client
    participant UI
    participant Client
end
box Server 
    participant RestApi
    participant DB
end

UI ->>+ Client: Login
Client ->> RestApi: Send Login Request
alt with username
    RestApi ->> DB: Check DB for User
    DB -->> RestApi: Query result
    alt exists
        RestApi ->> Client: Invalid Credentials
    else doesnt exist
        RestApi ->> Client: Token for temporary user With username
    end
else with username and password
    RestApi ->> DB: Check Credentials for existing user
    DB -->> RestApi: Query result
    alt exists and password is correct
        RestApi ->> Client: Token for existing User
    else exist and password is incorrect
        RestApi ->> Client: Invalid Credentials
    else doesnt exist
        RestApi ->> DB: Create new user with those credentials
        DB -->> RestApi: Query result
        RestApi ->> Client: new Token for the now existing User
    end
else without credentials
    RestApi ->> Client: Token for anonym User
end
Client ->>- UI: Display Login status


```

### Chatting

```mermaid
sequenceDiagram
box Client
    participant UI
    participant Client
end
box Server 
    participant RestApi
    participant DB
end

UI ->>+ Client: Join
loop    
    alt Normal
        par Message Receival
            Client ->>+ RestApi: get /room/<room-name>[?timeout=<timeout-in-seconds>]
            RestApi -->> Client: Start streamed responce
            RestApi -->> Client: Messages
            RestApi ->>- Client: responce end after timeout
        and Message Sending
                UI ->> Client: Enter Message
                Client ->>+ RestApi: post /room/<room-name>
                RestApi ->>- Client: Success or Error
        end
    else Leave
        UI ->> Client: Leave
        Client ->>+ RestApi: terminate connection
        RestApi -->>- Client: Ok
    end
end
Client ->>- UI: Session Terminated
```

