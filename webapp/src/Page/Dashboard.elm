module Page.Dashboard exposing (Model, Msg, init, update, view)

import Html exposing (Html, div, h1, h2, button, text, ul, li)
import Html.Attributes exposing (class)
import Html.Events exposing (onClick)
import Session exposing (Session)


type alias Model =
    { session : Session
    , projects : List ProjectSummary
    }


type alias ProjectSummary =
    { id : String
    , name : String
    }


type Msg
    = CreateProject
    | NoOp


init : Session -> ( Model, Cmd Msg )
init session =
    ( { session = session
      , projects = []
      }
    , Cmd.none
    )


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        CreateProject ->
            ( model, Cmd.none )

        NoOp ->
            ( model, Cmd.none )


view : Model -> Html Msg
view model =
    div [ class "container mx-auto p-8" ]
        [ h1 [ class "text-3xl font-bold text-white mb-6" ]
            [ text "Dashboard" ]
        , button
            [ class "bg-green-500 hover:bg-green-600 text-white px-4 py-2 rounded mb-6"
            , onClick CreateProject
            ]
            [ text "New Project" ]
        , h2 [ class "text-xl text-gray-300 mb-4" ] [ text "Your Projects" ]
        , if List.isEmpty model.projects then
            div [ class "text-gray-500" ] [ text "No projects yet" ]
          else
            ul [ class "space-y-2" ]
                (List.map viewProject model.projects)
        ]


viewProject : ProjectSummary -> Html Msg
viewProject project =
    li [ class "bg-gray-800 p-4 rounded" ]
        [ text project.name ]
