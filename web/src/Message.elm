module Message exposing (..)

import Json.Decode as Decode exposing (Decoder, list, string)
import Json.Decode.Pipeline exposing (required)
import Json.Encode as Encode


type alias Message =
    { name : String
    , value : String
    }


decodeList : Decoder (List Message)
decodeList =
    Decode.list decode


decode : Decoder Message
decode =
    Decode.succeed Message
        |> Json.Decode.Pipeline.required "name" Decode.string
        |> Json.Decode.Pipeline.required "value" Decode.string


toJson msg =
    Encode.object [ ( "name", Encode.string msg.name ), ( "value", Encode.string msg.value ) ]
