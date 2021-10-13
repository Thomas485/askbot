module Main exposing (..)

import Alert
import Bootstrap.Alert as Alert
import Bootstrap.Button as Button
import Bootstrap.ButtonGroup as BG
import Bootstrap.CDN as CDN
import Bootstrap.Form.Checkbox as Checkbox
import Bootstrap.Form.Fieldset as Fieldset
import Bootstrap.Form.Input as Input
import Bootstrap.Form.InputGroup as InputGroup
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
import Settings exposing (Settings)
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
    , login : Bool
    , loginKey : String
    , alerts : Alert.Alerts Msg
    , settings : Settings
    , credentialsChanged : Bool
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
        False
        loginKey
        (Alert.new RemoveAlert)
        { channel = ""
        , username = ""
        , oauth = ""
        , messageSuccess = ""
        , messageFailure = ""
        , reply = True
        }
        False
    , Requests.post { base_url = base_url } Login "login" <| loginJson loginKey
    )


type Msg
    = Tags (Result Http.Error (List Tag))
    | Tag TagAction (Result Http.Error ())
    | Messages (Result Http.Error (List Message))
    | Settings (Result Http.Error Settings)
    | SettingsUpdated (Result Http.Error ())
    | UpdatedMessage (Result Http.Error ())
    | Login (Result Http.Error ())
    | SendLogin String
    | RemoveTag Int
    | UpdateTag Int Tag
    | UpdateSettings Settings
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
    | RemoveAlert Int Alert.Visibility
    | UpdateSettingsText String String
    | UpdateSettingsReply Bool


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        Login (Ok _) ->
            ( { model | login = True, alerts = Alert.new model.alerts.alertAction }
            , Cmd.batch
                [ Requests.get model Tags Tag.decodeList "tags/"
                , Requests.get model Messages Message.decodeList "messages/"
                , Requests.get model Settings Settings.decode "settings/"
                ]
            )

        Login (Err e) ->
            let
                newModel =
                    Alert.add model Alert.dismissableAlert <|
                        "Login failed: "
                            ++ Error.toString e
            in
            ( { newModel
                | login = False
              }
            , Cmd.none
            )

        Tags (Ok ls) ->
            ( { model | tags = ls }
            , Cmd.none
            )

        Tags (Err e) ->
            ( Alert.add model Alert.dismissableAlert <|
                "Can't load tags: "
                    ++ Error.toString e
            , Cmd.none
            )

        Tag action (Ok _) ->
            let
                newModel =
                    Alert.add model Alert.dismissableSuccess <| Tag.successMessage action

                tags =
                    Tag.applyAction action model.tags
            in
            ( { newModel | tags = tags }
            , Cmd.none
            )

        Tag action (Err e) ->
            ( Alert.add model Alert.dismissableAlert <|
                Tag.failureMessage action e
            , Cmd.none
            )

        Messages (Ok ls) ->
            ( { model | messages = ls }
            , Cmd.none
            )

        Messages (Err e) ->
            ( Alert.add model Alert.dismissableAlert <|
                "Can't load messages: "
                    ++ Error.toString e
            , Cmd.none
            )

        Settings (Ok s) ->
            ( { model | settings = s }
            , Cmd.none
            )

        Settings (Err e) ->
            ( Alert.add model Alert.dismissableAlert <|
                "Can't load settings: "
                    ++ Error.toString e
            , Cmd.none
            )

        SettingsUpdated (Ok _) ->
            ( { model | credentialsChanged = False }
            , Cmd.none
            )

        SettingsUpdated (Err e) ->
            ( Alert.add model Alert.dismissableAlert <|
                "Can't save settings: "
                    ++ Error.toString e
            , Cmd.none
            )

        UpdatedMessage (Ok t) ->
            ( Alert.add model Alert.dismissableSuccess <|
                "Message sucessfully updated"
            , Cmd.none
            )

        UpdatedMessage (Err e) ->
            ( Alert.add model Alert.dismissableAlert <|
                "Can't update message on Server: "
                    ++ Error.toString e
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

        UpdateSettings s ->
            ( model
            , Requests.post model SettingsUpdated "settings/" <| Settings.toJson s
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

        RemoveAlert idx vis ->
            ( Alert.remove model idx
            , Cmd.none
            )

        UpdateSettingsText n t ->
            let
                ( credentialsChanged, settings ) =
                    updateSettings model.settings n t
            in
            ( { model | settings = settings, credentialsChanged = credentialsChanged }
            , Cmd.none
            )

        UpdateSettingsReply r ->
            let
                settings =
                    model.settings
            in
            ( { model | settings = { settings | reply = r } }
            , Cmd.none
            )


updateSettings settings name value =
    case name of
        "channel" ->
            ( True, { settings | channel = value } )

        "username" ->
            ( True, { settings | username = value } )

        "oauth" ->
            ( True, { settings | oauth = value } )

        "messageSuccess" ->
            ( False, { settings | messageSuccess = value } )

        "messageFailure" ->
            ( False, { settings | messageFailure = value } )

        _ ->
            ( False, settings )


tagPanel model =
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


textInputSection updater length name value =
    Html.p []
        --(tooltipMaxLength value length)
        [ InputGroup.config
            (InputGroup.text <|
                [ Input.onInput updater
                , Input.value value
                ]
             --++ markMaxLength value length
            )
            |> InputGroup.predecessors
                [ InputGroup.span []
                    [ text <|
                        name

                    --       ++ " ("
                    --       ++ String.fromInt (String.length value)
                    --       ++ "/"
                    --       ++ String.fromInt length
                    --       ++ ")"
                    ]
                ]
            |> InputGroup.view
        ]


checkboxSection updater name value =
    Html.p []
        [ Checkbox.advancedCheckbox
            [ Checkbox.id "reply_checkbox"
            , Checkbox.checked value
            , Checkbox.onCheck updater
            ]
            (Checkbox.label [] [ text name ])
        ]


settingsPanel model =
    Html.div []
        [ Fieldset.config
            |> Fieldset.asGroup
            |> Fieldset.legend [] [ text "Credentials:" ]
            |> Fieldset.children
                [ textInputSection
                    (UpdateSettingsText "channel")
                    25
                    "The channel to join"
                    model.settings.channel
                , textInputSection
                    (UpdateSettingsText "username")
                    25
                    "username"
                    model.settings.username
                , textInputSection
                    (UpdateSettingsText "oauth")
                    25
                    "oauth token"
                    model.settings.oauth
                ]
            |> Fieldset.view
        , Fieldset.config
            |> Fieldset.asGroup
            |> Fieldset.legend [] [ text "Chat-Messages:" ]
            |> Fieldset.children
                [ textInputSection
                    (UpdateSettingsText "messageSuccess")
                    25
                    "Response message on success"
                    model.settings.messageSuccess
                , textInputSection
                    (UpdateSettingsText "messageFailure")
                    25
                    "Response message on failure"
                    model.settings.messageFailure
                ]
            |> Fieldset.view
        , Fieldset.config
            |> Fieldset.asGroup
            |> Fieldset.legend [] [ text "Behavior:" ]
            |> Fieldset.children
                [ checkboxSection
                    UpdateSettingsReply
                    "use Reply-Messages"
                    model.settings.reply
                ]
            |> Fieldset.view
        , Button.button
            [ if model.credentialsChanged then
                Button.warning

              else
                Button.primary
            , Button.onClick <| UpdateSettings model.settings
            ]
            [ if model.credentialsChanged then
                text "save (and restart the bot please)"

              else
                text "save"
            ]
        ]


messageListHead =
    Table.simpleThead
        [ Table.th [ Table.cellAttr <| style "width" "15%" ] [ text "Type" ]
        , Table.th [ Table.cellAttr <| style "width" "100%" ] [ text "Message" ]
        , Table.th [] [ text "Action" ]
        ]


renameMessageName name =
    case name of
        "response_message_success" ->
            "Response message on success"

        "response_message_failure" ->
            "Response message on failure"

        _ ->
            name


messageRow i msg =
    Table.tr []
        [ Table.td []
            [ Html.label
                [ attribute "for" msg.name ]
                [ text <| renameMessageName msg.name ]
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
                [ tab "Tags" <| tagPanel model
                , tab "Settings" <| settingsPanel model
                ]
            |> Tab.view model.tabState
        ]
    ]


loginScreen model =
    [ Html.h2 [] [ text "Login:" ]
    , Html.label [ attribute "for" "key" ] [ text "Key (default: askbot):" ]
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
            :: Alert.panel Alert.defaultStyle model
            :: (if model.login then
                    mainPanel model

                else
                    loginScreen model
               )
    }
