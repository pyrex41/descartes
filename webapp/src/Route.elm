module Route exposing (Route(..), fromUrl, toString)

import Url exposing (Url)
import Url.Parser exposing (Parser, (</>), map, oneOf, s, string, top)


type Route
    = Home
    | Dashboard
    | Project String


parser : Parser (Route -> a) a
parser =
    oneOf
        [ map Home top
        , map Dashboard (s "dashboard")
        , map Project (s "project" </> string)
        ]


fromUrl : Url -> Maybe Route
fromUrl url =
    Url.Parser.parse parser url


toString : Route -> String
toString route =
    case route of
        Home ->
            "/"

        Dashboard ->
            "/dashboard"

        Project id ->
            "/project/" ++ id
