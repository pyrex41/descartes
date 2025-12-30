module Page.Project exposing (Model, Msg, init, update, view, subscriptions, handleEvent)

import Html exposing (Html, div, h1, h2, text, pre)
import Html.Attributes exposing (class)
import Session exposing (Session)
import Api exposing (Event)


type alias Model =
    { session : Session
    , projectId : String
    , name : String
    , events : List String
    }


type Msg
    = GotEvent String
    | NoOp


init : Session -> String -> ( Model, Cmd Msg )
init session projectId =
    ( { session = session
      , projectId = projectId
      , name = "Project " ++ projectId
      , events = []
      }
    , Cmd.none
    )


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotEvent event ->
            ( { model | events = event :: model.events }, Cmd.none )

        NoOp ->
            ( model, Cmd.none )


handleEvent : Event -> Model -> ( Model, Cmd Msg )
handleEvent event model =
    case event of
        Api.AgentSpawned agentId ->
            ( { model | events = ("Agent spawned: " ++ agentId) :: model.events }, Cmd.none )

        Api.AgentCompleted agentId ->
            ( { model | events = ("Agent completed: " ++ agentId) :: model.events }, Cmd.none )

        Api.TaskUpdated taskId status ->
            ( { model | events = ("Task " ++ taskId ++ ": " ++ status) :: model.events }, Cmd.none )


subscriptions : Model -> Sub Msg
subscriptions _ =
    Sub.none


view : Model -> Html Msg
view model =
    div [ class "container mx-auto p-8" ]
        [ h1 [ class "text-3xl font-bold text-white mb-6" ]
            [ text model.name ]
        , h2 [ class "text-xl text-gray-300 mb-4" ] [ text "Events" ]
        , pre [ class "bg-gray-800 p-4 rounded text-gray-300 text-sm" ]
            [ text (String.join "\n" model.events) ]
        ]
