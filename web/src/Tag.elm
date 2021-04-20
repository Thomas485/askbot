module Tag exposing (..)

import Error
import Http exposing (Error(..))
import Json.Decode as Decode exposing (Decoder, list, string)
import Json.Decode.Pipeline exposing (required)
import Json.Encode as Encode
import List.Extra exposing (removeAt, setAt, updateAt, updateIf)


type alias Tag =
    { tag : String
    , webhook : String
    }


decodeList : Decoder (List Tag)
decodeList =
    Decode.list decode


decode : Decoder Tag
decode =
    Decode.succeed Tag
        |> Json.Decode.Pipeline.required "tag" Decode.string
        |> Json.Decode.Pipeline.required "webhook" Decode.string


toJson tag =
    Encode.object [ ( "tag", Encode.string tag.tag ), ( "webhook", Encode.string tag.webhook ) ]


new : String -> String -> Tag
new ntag hook =
    { tag = ntag, webhook = hook }


type TagAction
    = Add Tag
    | Remove String Int
    | Update Tag Int


applyAction : TagAction -> List Tag -> List Tag
applyAction action ls =
    case action of
        Add t ->
            ls ++ [ t ]

        Remove name i ->
            removeAt i ls

        Update t i ->
            setAt i t ls


successMessage : TagAction -> String
successMessage action =
    case action of
        Add t ->
            "Tag \"" ++ t.tag ++ "\" added."

        Remove name _ ->
            "Tag \"" ++ name ++ "\" removed."

        Update t _ ->
            "Tag \"" ++ t.tag ++ "\" updated."


failureMessage : TagAction -> Http.Error -> String
failureMessage action e =
    "Can't "
        ++ (case action of
                Add t ->
                    "add Tag \"" ++ t.tag ++ "\""

                Remove name _ ->
                    "remove Tag \"" ++ name ++ "\""

                Update t _ ->
                    "update Tag \"" ++ t.tag ++ "\""
           )
        ++ ": "
        ++ Error.toString e
