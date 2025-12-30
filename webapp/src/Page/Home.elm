module Page.Home exposing (Model, Msg, init, update, view)

import Html exposing (Html, div, h1, p, a, text)
import Html.Attributes exposing (class, href)
import Session exposing (Session)


type alias Model =
    { session : Session
    }


type Msg
    = NoOp


init : Session -> ( Model, Cmd Msg )
init session =
    ( { session = session }, Cmd.none )


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        NoOp ->
            ( model, Cmd.none )


view : Model -> Html Msg
view _ =
    div [ class "container mx-auto p-8" ]
        [ h1 [ class "text-4xl font-bold text-white mb-4" ]
            [ text "Descartes" ]
        , p [ class "text-gray-300 mb-8" ]
            [ text "Guided software development with AI agents" ]
        , a [ href "/dashboard", class "bg-blue-500 hover:bg-blue-600 text-white px-6 py-3 rounded" ]
            [ text "Go to Dashboard" ]
        ]
