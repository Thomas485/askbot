module Main exposing (..)

import Bootstrap.Alert as Alert
import Bootstrap.Button as Button
import Bootstrap.ButtonGroup as BG
import Bootstrap.CDN as CDN
import Bootstrap.Form.Input as Input
import Bootstrap.Tab as Tab
import Bootstrap.Table as Table
import Browser
import Browser.Navigation as Nav
import Error
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick, onInput)
import Http exposing (Error(..))
import Json.Encode as Encode
import List.Extra exposing (removeAt, setAt, updateAt, updateIf)
import Message exposing (Message)
import Requests
import Tag exposing (Tag, TagAction(..))
import Url


main : Program () Model Msg
main =
    Browser.application
        { init = init
        , view = view
        , update = update
        , subscriptions = \_ -> Sub.none
        , onUrlChange = \_ -> NoMsg
        , onUrlRequest = \_ -> NoMsg
        }


type alias Model =
    { url : Url.Url
    , base_url : String
    , tags : List Tag
    , messages : List Message
    , newTag : Tag
    , tabState : Tab.State
    , alert : Maybe (List (Html Msg))
    , login : Bool
    , loginKey : String
    }


loginJson key =
    Encode.object [ ( "key", Encode.string key ) ]


extractKey : String -> String
extractKey queryString =
    let
        h ss =
            case ss of
                [ "key", k ] :: _ ->
                    k

                _ :: sss ->
                    h sss

                [] ->
                    ""
    in
    h <|
        List.map (String.split "=") <|
            String.split "&" queryString


init : () -> Url.Url -> Nav.Key -> ( Model, Cmd Msg )
init _ url _ =
    let
        base_url =
            Url.toString { url | path = "/", query = Nothing, fragment = Nothing }

        loginKey =
            extractKey <| Maybe.withDefault "" url.query
    in
    ( Model url
        base_url
        []
        []
        (Tag.new "" "")
        Tab.initialState
        Nothing
        False
        loginKey
    , Requests.post { base_url = base_url } Login "login" <| loginJson loginKey
    )


type Msg
    = Tags (Result Http.Error (List Tag))
    | Tag TagAction (Result Http.Error ())
    | Messages (Result Http.Error (List Message))
    | UpdatedMessage (Result Http.Error ())
    | Login (Result Http.Error ())
    | SendLogin String
    | RemoveTag Int
    | UpdateTag Int Tag
    | UpdateMessageText String String
    | UpdateMessage String String
    | UpdateExistingTag Int String
    | UpdateExistingWebhook Int String
    | UpdateNewTag String
    | UpdateNewWebhook String
    | AddTag Tag
    | TabMsgMain Tab.State
    | UpdateLoginKey String
    | NoMsg


alertStyle =
    [ style "margin" "10px", style "margin-bottom" "0px" ]


add_alert : Model -> String -> Maybe (List (Html Msg))
add_alert model msg =
    Just <|
        Alert.simpleDanger alertStyle [ text msg ]
            :: Maybe.withDefault [] model.alert


add_success : Model -> String -> Maybe (List (Html Msg))
add_success model msg =
    Just <|
        Alert.simpleSuccess alertStyle [ text msg ]
            :: Maybe.withDefault [] model.alert


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        Login (Ok _) ->
            ( { model | login = True }
            , Cmd.batch
                [ Requests.get model Tags Tag.decodeList "tags/"
                , Requests.get model Messages Message.decodeList "messages/"
                ]
            )

        Login (Err e) ->
            ( { model
                | login = False
                , alert = add_alert model <| "Login failed: " ++ Error.toString e
              }
            , Cmd.none
            )

        Tags (Ok ls) ->
            ( { model | tags = ls, alert = Nothing }
            , Cmd.none
            )

        Tags (Err e) ->
            ( { model
                | alert = add_alert model <| "Can't load tags: " ++ Error.toString e
              }
            , Cmd.none
            )

        Tag action (Ok _) ->
            let
                alerts =
                    add_success model <| Tag.successMessage action

                tags =
                    Tag.applyAction action model.tags
            in
            ( { model | tags = tags, alert = alerts }
            , Cmd.none
            )

        Tag action (Err e) ->
            let
                alerts =
                    add_alert model <| Tag.failureMessage action e
            in
            ( { model | alert = alerts }
            , Cmd.none
            )

        Messages (Ok ls) ->
            ( { model | messages = ls, alert = Nothing }
            , Cmd.none
            )

        Messages (Err e) ->
            ( { model
                | alert = add_alert model <| "Can't load messages: " ++ Error.toString e
              }
            , Cmd.none
            )

        UpdatedMessage (Ok t) ->
            ( { model | alert = add_success { model | alert = Nothing } <| "Message sucessfully updated" }, Cmd.none )

        UpdatedMessage (Err e) ->
            ( { model
                | alert = add_alert model <| "Can't update message on Server: " ++ Error.toString e
              }
            , Cmd.none
            )

        UpdateNewTag t ->
            ( { model | newTag = Tag.new t model.newTag.webhook }, Cmd.none )

        UpdateNewWebhook w ->
            ( { model | newTag = Tag.new model.newTag.tag w }, Cmd.none )

        UpdateExistingTag i t ->
            ( { model | tags = updateAt i (\tag -> { tag | tag = t }) model.tags }
            , Cmd.none
            )

        UpdateExistingWebhook i w ->
            ( { model | tags = updateAt i (\tag -> { tag | webhook = w }) model.tags }
            , Cmd.none
            )

        UpdateMessageText name value ->
            ( { model
                | messages =
                    updateIf
                        (\oldMsg -> oldMsg.name == name)
                        (\oldMsg -> { oldMsg | value = value })
                        model.messages
              }
            , Cmd.none
              -- TODO: add a warning, if the text is longer than 480 characters.
              -- 510 (max message) - 25 (Name length) - 5 (misc. like @: etc.) = 480
            )

        UpdateMessage name message ->
            ( model
            , Requests.post model UpdatedMessage ("messages/" ++ name) <| Encode.string message
            )

        RemoveTag i ->
            let
                tag =
                    .tag <| Maybe.withDefault { tag = "", webhook = "" } <| List.Extra.getAt i model.tags
            in
            ( model
            , Requests.delete model (Tag <| Remove tag i) "tags/" i
            )

        UpdateTag i t ->
            ( model
            , Requests.put model (Tag <| Update t i) "tags/" i <| Tag.toJson t
            )

        AddTag t ->
            ( { model | newTag = { tag = "", webhook = "" } }
            , Requests.post model (Tag <| Add t) "tags/add" <| Tag.toJson model.newTag
            )

        NoMsg ->
            ( model, Cmd.none )

        TabMsgMain state ->
            ( { model | tabState = state }
            , Cmd.none
            )

        UpdateLoginKey k ->
            ( { model | loginKey = k }
            , Cmd.none
            )

        SendLogin key ->
            ( model
            , Requests.post model Login "login" <| loginJson key
            )



-- VIEW


tagTable model =
    Table.table { options = [ Table.small, Table.responsive ], thead = tagListHead, tbody = tagListBody model }


tagListHead =
    Table.simpleThead
        [ Table.th [ Table.cellAttr <| style "width" "15%" ] [ text "Tag" ]
        , Table.th [ Table.cellAttr <| style "width" "100%" ] [ text "Webhook" ]
        , Table.th [] [ text "Action" ]
        ]


tagActions i tv wv =
    [ BG.buttonGroup []
        [ BG.button
            [ Button.primary
            , Button.onClick <| RemoveTag i
            ]
            [ text "delete" ]
        , BG.button
            [ Button.primary
            , Button.onClick <| UpdateTag i { tag = tv, webhook = wv }
            ]
            [ text "update" ]
        ]
    ]


tagRow i tid wid tv wv =
    Table.tr []
        [ Table.td []
            [ Input.text
                [ Input.id tid
                , Input.value tv
                , Input.onInput <| UpdateExistingTag i
                ]
            ]
        , Table.td []
            [ Input.url
                [ Input.id wid
                , Input.value wv
                , Input.onInput <| UpdateExistingWebhook i
                ]
            ]
        , Table.td []
            (tagActions
                i
                tv
                wv
            )
        ]


tagListBody model =
    Table.tbody []
        (List.indexedMap (\i t -> tagRow i ("tag" ++ String.fromInt i) ("hook" ++ String.fromInt i) t.tag t.webhook) model.tags
            ++ [ Table.tr []
                    [ Table.td []
                        [ Input.text
                            [ Input.id "new_tag"
                            , Input.value model.newTag.tag
                            , Input.onInput UpdateNewTag
                            ]
                        ]
                    , Table.td []
                        [ Input.url
                            [ Input.id "new_webhook"
                            , Input.value model.newTag.webhook
                            , Input.onInput UpdateNewWebhook
                            ]
                        ]
                    , Table.td []
                        [ Button.button
                            [ Button.primary
                            , Button.onClick <| AddTag model.newTag
                            ]
                            [ text "add" ]
                        ]
                    ]
               ]
        )


messageActions name value =
    [ BG.buttonGroup []
        [ BG.button
            [ Button.primary
            , Button.onClick <| UpdateMessage name value
            ]
            [ text "update" ]
        ]
    ]


messageTable model =
    Table.table { options = [ Table.small ], thead = messageListHead, tbody = messageListBody model }


messageListHead =
    Table.simpleThead
        [ Table.th [ Table.cellAttr <| style "width" "15%" ] [ text "Type" ]
        , Table.th [ Table.cellAttr <| style "width" "100%" ] [ text "Message" ]
        , Table.th [] [ text "Action" ]
        ]


messageRow i msg =
    Table.tr []
        [ Table.td []
            [ Html.label
                [ attribute "for" msg.name ]
                [ text msg.name ]
            ]
        , Table.td []
            [ Input.text
                [ Input.id msg.name
                , Input.value msg.value
                , Input.onInput <| UpdateMessageText msg.name
                ]
            ]
        , Table.td [] <| messageActions msg.name msg.value
        ]


messageListBody model =
    Table.tbody [] <| List.indexedMap messageRow model.messages


tab name content =
    Tab.item
        { id = name
        , link = Tab.link [] [ text name ]
        , pane =
            Tab.pane [ style "padding" "10px" ]
                [ h4 []
                    [ text <| name ++ ":" ]
                , div [] [ content ]
                ]
        }


mainPanel model =
    [ div
        [ style "padding" "10px" ]
        [ Tab.config TabMsgMain
            |> Tab.pills
            |> Tab.useHash True
            |> Tab.items
                [ tab "Tags" <| tagTable model
                , tab "Messages" <| messageTable model
                , tab "Settings" <| p [] [ text "nothing" ]
                ]
            |> Tab.view model.tabState
        ]
    ]


loginScreen model =
    [ Html.h2 [] [ text "Login:" ]
    , Html.label [ attribute "for" "key" ] [ text "Key:" ]
    , Input.text
        [ Input.id "key"
        , Input.value model.loginKey
        , Input.onInput UpdateLoginKey
        ]
    , Button.button
        [ Button.primary
        , Button.onClick <| SendLogin model.loginKey
        ]
        [ text "login" ]
    ]


view : Model -> Browser.Document Msg
view model =
    { title = "Bot"
    , body =
        CDN.stylesheet
            :: Maybe.withDefault [] model.alert
            ++ (if model.login then
                    mainPanel model

                else
                    loginScreen model
               )
    }
