module Page.NotFound exposing (view)

import Html exposing (Html, div, h1, p, a, text)
import Html.Attributes exposing (class, href)


view : Html msg
view =
    div [ class "container mx-auto p-8 text-center" ]
        [ h1 [ class "text-4xl font-bold text-white mb-4" ]
            [ text "404" ]
        , p [ class "text-gray-300 mb-8" ]
            [ text "Page not found" ]
        , a [ href "/", class "text-blue-400 hover:text-blue-300" ]
            [ text "Go home" ]
        ]
