module Main exposing (..)

import Browser
import Browser.Navigation as Nav
import Html exposing (..)
import Html.Attributes exposing (..)
import Url
import Url.Parser exposing (Parser, parse, string)
import Json.Decode as Decode exposing (Decoder, string, list)
import Json.Decode.Pipeline exposing (required, optional, hardcoded)
import Http exposing (get,Error(..))
import List.Extra exposing (removeAt)
import Html.Events exposing (onClick,onInput)


parseError : String -> Maybe String
parseError =
  Decode.decodeString (Decode.field "error" Decode.string) >> Result.toMaybe


errorToString : Maybe Http.Error -> String
errorToString err =
  case err of
    Just Timeout ->
      "Error: Timeout exceeded"

    Just NetworkError ->
      "Error: Network error"

    Just (BadStatus c) ->
      "Error: Bad status code: " ++ String.fromInt c

    Just (BadBody text) ->
      "Error: Unexpected response from api: " ++ text

    Just (BadUrl url) ->
      "Error: Malformed url: " ++ url

    Nothing -> 
      ""


encode = Url.percentEncode

-- MAIN


main : Program () Model Msg
main =
  Browser.application
    { init = init
    , view = view
    , update = update
    , subscriptions = subscriptions
    , onUrlChange = \_->NoMsg
    , onUrlRequest = \_->NoMsg
    }


tagsDecoder : Decoder (List Tag)
tagsDecoder = Decode.list tagDecoder

tagDecoder : Decoder Tag
tagDecoder =
  Decode.succeed Tag
    |> Json.Decode.Pipeline.required "tag" Decode.string
    |> Json.Decode.Pipeline.required "webhook" Decode.string


-- MODEL


type alias Model =
  { url : Url.Url
  , base_url: String
  , key: Maybe String
  , tags: List Tag
  , error: Maybe (Http.Error)
  , newTag: Tag
  }


init : () -> Url.Url -> Nav.Key -> ( Model, Cmd Msg )
init flags url _ =
  let
    path = url.path
    key = Url.Parser.parse Url.Parser.string url
    base_url = Url.toString {url | path="/", query=Nothing,fragment=Nothing}
  in
  ( Model url base_url key [] Nothing {tag="",webhook=""}
  , Http.get
      { url = base_url ++"tags/" ++ encode (Maybe.withDefault "noKey" key)
      , expect = Http.expectJson Tags tagsDecoder
      }
  )



-- UPDATE

type alias Tag = 
  { tag: String
  , webhook: String
  }

type Msg
  = Tags (Result Http.Error (List Tag))
  | RemovedTag (Result Http.Error Tag)
  | RemoveTag Int
  | UpdateNewTag String
  | UpdateNewWebhook String
  | AddTag Tag
  | NoMsg


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
  case msg of
    Tags (Ok ls) ->
      ( { model | tags = ls}
      , Cmd.none
      )
    Tags (Err e) ->
      ( {model | error =  Just e}
      , Cmd.none
      )
    RemovedTag (Err e) ->
      ( {model | error =  Just e}
      , Cmd.none
      )
    RemovedTag _ -> ( model, Cmd.none)
    UpdateNewTag t ->
      ( {model | newTag = newTag t model.newTag.webhook}, Cmd.none)
    UpdateNewWebhook w ->
      ( {model | newTag = newTag model.newTag.tag w}, Cmd.none)
    RemoveTag i ->
      ( {model | tags =  removeAt i model.tags}
      , Http.get
          { url = model.base_url ++ "delete_tag/" ++ String.fromInt i ++ "/" ++ encode (Maybe.withDefault "noKey" model.key)
          , expect = Http.expectJson RemovedTag tagDecoder
          }
      )
    AddTag t ->
      ( model
      , Http.get
          { url = model.base_url ++ "add_tag/" ++ encode model.newTag.tag ++ "/" ++ encode model.newTag.webhook ++ "/" ++ encode (Maybe.withDefault "noKey" model.key)
          , expect = Http.expectJson Tags tagsDecoder
          }
      )
    NoMsg -> (model, Cmd.none)



-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions _ =
  Sub.none



-- VIEW

newTag: String -> String -> Tag
newTag ntag hook = {tag=ntag, webhook=hook}

view : Model -> Browser.Document Msg
view model =
  { title = "Bot"
  , body =
      [ h2 [style "margin" "10px"] [text "Tags:"]
      , model.tags |> List.indexedMap
        (\i t -> div [style "margin" "10px"]
          [ input
            [ value t.tag
            , disabled True
            ]
            []
          , input
            [ value t.webhook
            , style "width" "50%"
            , disabled True
            ]
            []
          , button [onClick (RemoveTag i)] [text "delete"]
          ]) |> div []
      , div [style "margin" "10px"]
        [ input
          [ placeholder "tag"
          , value model.newTag.tag
          , onInput (\t-> UpdateNewTag t)
          ]
          []
        , input
          [ style "width" "50%"
          , placeholder "webhook URL"
          , value model.newTag.webhook
          , onInput (\t-> UpdateNewWebhook t)
          ]
          []
        , button [onClick (AddTag model.newTag)] [text "add"]
        ]
      , div [style "margin" "10px"] [text (errorToString model.error)]
      ]
  }
