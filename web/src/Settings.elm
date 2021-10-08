module Settings exposing (..)

import Json.Decode as Decode exposing (Decoder, bool, list, string)
import Json.Decode.Pipeline exposing (required)
import Json.Encode as Encode


type alias Settings =
    { channel : String
    , username : String
    , oauth : String
    , messageSuccess : String
    , messageFailure : String
    , reply : Bool
    }


decode : Decoder Settings
decode =
    Decode.succeed Settings
        |> Json.Decode.Pipeline.required "channel" Decode.string
        |> Json.Decode.Pipeline.required "username" Decode.string
        |> Json.Decode.Pipeline.required "oauth" Decode.string
        |> Json.Decode.Pipeline.required "messageSuccess" Decode.string
        |> Json.Decode.Pipeline.required "messageFailure" Decode.string
        |> Json.Decode.Pipeline.required "reply" Decode.bool


toJson settings =
    Encode.object
        [ ( "channel", Encode.string settings.channel )
        , ( "username", Encode.string settings.username )
        , ( "oauth", Encode.string settings.oauth )
        , ( "messageSuccess", Encode.string settings.messageSuccess )
        , ( "messageFailure", Encode.string settings.messageFailure )
        , ( "reply", Encode.bool settings.reply )
        ]
