module Session exposing (Session, guest, authenticated, isAuthenticated, getToken)


type Session
    = Guest
    | Authenticated String


guest : Session
guest =
    Guest


authenticated : String -> Session
authenticated token =
    Authenticated token


isAuthenticated : Session -> Bool
isAuthenticated session =
    case session of
        Guest ->
            False

        Authenticated _ ->
            True


getToken : Session -> Maybe String
getToken session =
    case session of
        Guest ->
            Nothing

        Authenticated token ->
            Just token
