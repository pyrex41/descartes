module Api exposing (Event(..), decodeEvent, eventsUrl)

import Json.Decode as Decode exposing (Decoder)
import Session exposing (Session)


type Event
    = AgentSpawned String
    | AgentCompleted String
    | TaskUpdated String String


decodeEvent : String -> Result Decode.Error Event
decodeEvent json =
    Decode.decodeString eventDecoder json


eventDecoder : Decoder Event
eventDecoder =
    Decode.field "type" Decode.string
        |> Decode.andThen eventTypeDecoder


eventTypeDecoder : String -> Decoder Event
eventTypeDecoder eventType =
    case eventType of
        "agent_spawned" ->
            Decode.map AgentSpawned (Decode.field "agent_id" Decode.string)

        "agent_completed" ->
            Decode.map AgentCompleted (Decode.field "agent_id" Decode.string)

        "task_updated" ->
            Decode.map2 TaskUpdated
                (Decode.field "task_id" Decode.string)
                (Decode.field "status" Decode.string)

        _ ->
            Decode.fail ("Unknown event type: " ++ eventType)


eventsUrl : Session -> String
eventsUrl _ =
    "ws://localhost:8081/events"
