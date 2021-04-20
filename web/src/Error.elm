module Error exposing (..)

import Http exposing (Error(..))


toString : Http.Error -> String
toString err =
    case err of
        Timeout ->
            "Error: Timeout exceeded"

        NetworkError ->
            "Error: Network error"

        BadStatus c ->
            "Error: Bad status code: " ++ String.fromInt c

        BadBody text ->
            "Error: Unexpected response from api: " ++ text

        BadUrl url ->
            "Error: Malformed url: " ++ url
