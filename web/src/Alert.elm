module Alert exposing
    ( Alerts
    , HasAlert
    , add
    , defaultStyle
    , dismissableAlert
    , dismissableSuccess
    , new
    , panel
    , remove
    )

import Bootstrap.Alert as Alert
import Dict exposing (Dict)
import Html exposing (..)
import Html.Attributes exposing (style)


type alias AlertAction msg =
    Int -> Alert.Visibility -> msg


dismissable_alert_base : AlertAction msg -> (Alert.Config msg -> Alert.Config msg) -> Int -> String -> Html msg
dismissable_alert_base action alert id msg =
    Alert.config
        |> alert
        |> Alert.dismissable (action id)
        |> Alert.children [ Html.text msg ]
        |> Alert.view Alert.shown


dismissableAlert : HasAlert model msg -> Int -> String -> Html msg
dismissableAlert model =
    dismissable_alert_base model.alerts.alertAction Alert.danger


dismissableSuccess : HasAlert model msg -> Int -> String -> Html msg
dismissableSuccess model =
    dismissable_alert_base model.alerts.alertAction Alert.success


type alias Alerts msg =
    { alert : Dict Int (Html msg)
    , lastAlertId : Int
    , alertAction : AlertAction msg
    }


new : AlertAction msg -> Alerts msg
new action =
    { alert = Dict.empty
    , lastAlertId = 0
    , alertAction = action
    }


defaultStyle : List (Html.Attribute msg)
defaultStyle =
    [ style "margin" "10px", style "margin-bottom" "0px" ]


panel : List (Html.Attribute msg) -> HasAlert model msg -> Html msg
panel style model =
    Html.div style (Dict.values model.alerts.alert)


remove : HasAlert model msg -> Int -> HasAlert model msg
remove model idx =
    let
        alerts =
            model.alerts
    in
    { model | alerts = { alerts | alert = Dict.remove idx model.alerts.alert } }


type alias HasAlert model msg =
    { model
        | alerts : Alerts msg
    }


add : HasAlert model msg -> (HasAlert model msg -> Int -> String -> Html msg) -> String -> HasAlert model msg
add model alertType message =
    let
        alerts =
            model.alerts
    in
    { model
        | alerts =
            { alerts
                | alert =
                    Dict.insert model.alerts.lastAlertId
                        (alertType model model.alerts.lastAlertId message)
                        model.alerts.alert
                , lastAlertId = model.alerts.lastAlertId + 1
            }
    }


clear : HasAlert model msg -> HasAlert model msg
clear model =
    { model | alerts = new model.alerts.alertAction }
