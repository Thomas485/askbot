module Requests exposing (..)

import Http


get model msg decoder path =
    Http.get
        { url = model.base_url ++ path
        , expect = Http.expectJson msg decoder
        }


delete model msg path name =
    Http.request
        { method = "DELETE"
        , url = model.base_url ++ path ++ String.fromInt name
        , headers = []
        , body = Http.emptyBody
        , expect = Http.expectWhatever msg
        , timeout = Nothing
        , tracker = Nothing
        }


put model msg path name jsonValue =
    Http.request
        { method = "PUT"
        , url = model.base_url ++ path ++ String.fromInt name
        , headers = []
        , body = Http.jsonBody <| jsonValue
        , expect = Http.expectWhatever msg
        , timeout = Nothing
        , tracker = Nothing
        }


post model msg path jsonValue =
    Http.post
        { url = model.base_url ++ path
        , body = Http.jsonBody <| jsonValue
        , expect = Http.expectWhatever msg
        }
