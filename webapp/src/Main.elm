module Main exposing (main)

import Api
import Browser
import Browser.Navigation as Nav
import Html exposing (Html, div, text)
import Html.Attributes exposing (class)
import Page.Dashboard as Dashboard
import Page.Home as Home
import Page.NotFound as NotFound
import Page.Project as Project
import Ports
import Route exposing (Route)
import Session exposing (Session)
import Url exposing (Url)


main : Program () Model Msg
main =
    Browser.application
        { init = init
        , view = view
        , update = update
        , subscriptions = subscriptions
        , onUrlChange = UrlChanged
        , onUrlRequest = LinkClicked
        }



-- MODEL


type Model
    = Home Nav.Key Home.Model
    | Dashboard Nav.Key Dashboard.Model
    | Project Nav.Key Project.Model
    | NotFound Nav.Key Session


type alias Flags =
    ()


init : Flags -> Url -> Nav.Key -> ( Model, Cmd Msg )
init _ url key =
    changeRouteTo (Route.fromUrl url) (NotFound key Session.guest) key



-- UPDATE


type Msg
    = LinkClicked Browser.UrlRequest
    | UrlChanged Url
    | HomeMsg Home.Msg
    | DashboardMsg Dashboard.Msg
    | ProjectMsg Project.Msg
    | WebSocketMessageReceived String
    | WebSocketStatusChanged String


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case ( msg, model ) of
        ( LinkClicked urlRequest, _ ) ->
            case urlRequest of
                Browser.Internal url ->
                    ( model, Nav.pushUrl (toKey model) (Url.toString url) )

                Browser.External href ->
                    ( model, Nav.load href )

        ( UrlChanged url, _ ) ->
            changeRouteTo (Route.fromUrl url) model (toKey model)

        ( HomeMsg subMsg, Home key homeModel ) ->
            Home.update subMsg homeModel
                |> updateWith (Home key) HomeMsg

        ( DashboardMsg subMsg, Dashboard key dashboardModel ) ->
            Dashboard.update subMsg dashboardModel
                |> updateWith (Dashboard key) DashboardMsg

        ( ProjectMsg subMsg, Project key projectModel ) ->
            Project.update subMsg projectModel
                |> updateWith (Project key) ProjectMsg

        ( WebSocketMessageReceived message, Project key projectModel ) ->
            case Api.decodeEvent message of
                Ok event ->
                    Project.handleEvent event projectModel
                        |> updateWith (Project key) ProjectMsg

                Err _ ->
                    ( model, Cmd.none )

        ( WebSocketMessageReceived _, _ ) ->
            ( model, Cmd.none )

        ( WebSocketStatusChanged _, _ ) ->
            ( model, Cmd.none )

        _ ->
            ( model, Cmd.none )


updateWith : (subModel -> Model) -> (subMsg -> Msg) -> ( subModel, Cmd subMsg ) -> ( Model, Cmd Msg )
updateWith toModel toMsg ( subModel, subCmd ) =
    ( toModel subModel, Cmd.map toMsg subCmd )


changeRouteTo : Maybe Route -> Model -> Nav.Key -> ( Model, Cmd Msg )
changeRouteTo maybeRoute model key =
    let
        session =
            toSession model
    in
    case maybeRoute of
        Nothing ->
            ( NotFound key session, Cmd.none )

        Just Route.Home ->
            Home.init session
                |> updateWith (Home key) HomeMsg

        Just Route.Dashboard ->
            Dashboard.init session
                |> updateWith (Dashboard key) DashboardMsg

        Just (Route.Project projectId) ->
            Project.init session projectId
                |> updateWith (Project key) ProjectMsg



-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.batch
        [ Ports.webSocketMessage WebSocketMessageReceived
        , Ports.webSocketStatus WebSocketStatusChanged
        , case model of
            Project _ projectModel ->
                Sub.map ProjectMsg (Project.subscriptions projectModel)

            _ ->
                Sub.none
        ]



-- VIEW


view : Model -> Browser.Document Msg
view model =
    let
        viewPage toMsg content =
            { title = "Descartes"
            , body =
                [ div [ class "min-h-screen bg-gray-900" ]
                    [ Html.map toMsg content ]
                ]
            }
    in
    case model of
        Home _ homeModel ->
            viewPage HomeMsg (Home.view homeModel)

        Dashboard _ dashboardModel ->
            viewPage DashboardMsg (Dashboard.view dashboardModel)

        Project _ projectModel ->
            viewPage ProjectMsg (Project.view projectModel)

        NotFound _ _ ->
            { title = "404 - Descartes"
            , body = [ div [ class "min-h-screen bg-gray-900" ] [ NotFound.view ] ]
            }



-- HELPERS


toSession : Model -> Session
toSession model =
    case model of
        Home _ homeModel ->
            homeModel.session

        Dashboard _ dashboardModel ->
            dashboardModel.session

        Project _ projectModel ->
            projectModel.session

        NotFound _ session ->
            session


toKey : Model -> Nav.Key
toKey model =
    case model of
        Home key _ ->
            key

        Dashboard key _ ->
            key

        Project key _ ->
            key

        NotFound key _ ->
            key
