port module Ports exposing
    ( connectWebSocket
    , disconnectWebSocket
    , sendWebSocketMessage
    , webSocketMessage
    , webSocketStatus
    )

-- Outgoing ports (Elm -> JS)
port connectWebSocket : String -> Cmd msg
port disconnectWebSocket : () -> Cmd msg
port sendWebSocketMessage : String -> Cmd msg

-- Incoming ports (JS -> Elm)
port webSocketMessage : (String -> msg) -> Sub msg
port webSocketStatus : (String -> msg) -> Sub msg
