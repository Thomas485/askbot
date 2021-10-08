#![cfg_attr(feature = "webfrontend", feature(proc_macro_hygiene, decl_macro))]

use serde::{Deserialize, Serialize};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;
use twitch_irc::SecureTCPTransport;

use std::sync::{Arc, RwLock};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

#[cfg(feature = "webfrontend")]
#[macro_use]
extern crate rocket;

#[cfg(feature = "webfrontend")]
mod web;

mod generate;

type IRCClient = twitch_irc::TwitchIRCClient<SecureTCPTransport, StaticLoginCredentials>;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Msg {
    username: String,
    avatar_url: String,
    content: String,
}

fn msg(username: String, content: String) -> Msg {
    Msg {
        username,
        avatar_url: "".to_owned(),
        content,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Tag {
    tag: String,
    webhook: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BotConfig {
    channel: String,
    username: String,
    oauth_token: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    tags: Vec<Tag>,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    key: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    mods: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    log_webhook: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    response_message_success: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    response_message_failure: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    ignore: Vec<String>,
    #[serde(skip_serializing_if = "bool_id")]
    #[serde(default = "bool_true")]
    use_reply: bool,
}

fn bool_id(a: &bool) -> bool {
    *a
}

fn bool_true() -> bool {
    true
}

async fn say_in_response<T>(channel: String, client: &IRCClient, msg: T, reply_to: Option<String>)
where
    T: Into<String>,
{
    if let Err(e) = client.say_in_response(channel, msg.into(), reply_to).await {
        error!("Error: {}", e);
    }
}

async fn privmsg<T>(channel: String, client: &IRCClient, msg: T)
where
    T: Into<String>,
{
    match client.privmsg(channel, msg.into()).await {
        Ok(_) => (),
        Err(e) => error!("{}", e),
    };
}

pub fn read_config(config_file: &str) -> std::result::Result<BotConfig, serde_any::Error> {
    serde_any::from_file(config_file)
}

pub fn write_config_logged(config_file: &str, bc: &BotConfig) {
    match serde_any::to_file_pretty(config_file, bc) {
        Ok(()) => (),
        Err(e) => error!("Can't write config file {}: {}", config_file, e),
    }
}

fn is_mod(badges: &[twitch_irc::message::Badge]) -> bool {
    badges
        .iter()
        .any(|b| b.name == "moderator" || b.name == "broadcaster")
}

fn send_message(webhook: &str, sender: String, text: String) -> bool {
    let client = reqwest::blocking::Client::new();
    match client.post(webhook).json(&msg(sender, text)).send() {
        Ok(resp) => {
            if resp.status().is_success() {
                return true;
            } else {
                error!(
                    "Error: Code: {} Reason: {:?}",
                    resp.status(),
                    resp.status().canonical_reason()
                );
            }
        }
        Err(e) => {
            error!("Error: {}", e);
        }
    };
    false
}

async fn send_messages(
    irc_bc: &Arc<RwLock<BotConfig>>,
    message_text: String,
    sender: &twitch_irc::message::TwitchUserBasics,
    client: &IRCClient,
    message_id: String,
) {
    let mut sended = false;
    let mut success = true;
    {
        let bc = irc_bc.read().unwrap();
        for t in bc.tags.iter() {
            if t.tag.is_empty() {
                continue;
            }
            let text_lower = message_text.to_lowercase();
            if text_lower.contains(&(t.tag.to_lowercase() + " "))
                || text_lower.ends_with(&t.tag.to_lowercase())
            {
                success =
                    success && send_message(&t.webhook, sender.login.clone(), message_text.clone());
                sended = true;
            }
        }
    }
    let mut message: (String, String) = ("".to_string(), "".to_string());
    if sended {
        let bc = irc_bc.read().unwrap();
        if success {
            message = (bc.response_message_success.clone(), bc.channel.clone());
        } else {
            message = (bc.response_message_failure.clone(), bc.channel.clone());
        }
    }
    if !message.0.is_empty() {
        let reply = {
            let bc = irc_bc.read().unwrap();
            bc.use_reply
        };
        let msg = if reply {
            message.0
        } else {
            format!("@{}: {}", &sender.login, message.0)
        };
        let reply_id = if reply { Some(message_id) } else { None };
        say_in_response(message.1, client, msg, reply_id).await;
    }
}

enum Whisper {
    Add(String, String),
    Remove(String),
    List,
    Nothing,
}

fn parse_whisper(bc: &BotConfig, login: &str, message_text: String) -> Whisper {
    if bc.mods.iter().any(|m| *m == *login) || *login == bc.channel {
        match message_text.split_whitespace().collect::<Vec<&str>>()[..] {
            ["#list"] => Whisper::List,
            ["#add", tag, webhook] => Whisper::Add(tag.to_string(), webhook.to_string()),
            ["#remove", tag] => Whisper::Remove(tag.to_string()),
            _ => Whisper::Nothing,
        }
    } else {
        Whisper::Nothing
    }
}

fn join_tags(ts: &[Tag]) -> String {
    ts.iter()
        .map(|t| t.tag.clone())
        .collect::<Vec<String>>()
        .join(", ")
}

fn sender_is_ignored(irc_bc: &Arc<RwLock<BotConfig>>, sender: &str) -> bool {
    irc_bc
        .read()
        .unwrap()
        .ignore
        .iter()
        .any(|s| s.to_lowercase() == sender.to_lowercase())
}

fn handle_whisper(
    irc_bc: &Arc<RwLock<BotConfig>>,
    login: String,
    message_text: String,
    config_file: &str,
) -> Option<(String, String, String)> {
    let mut bc = irc_bc.write().unwrap();
    if !bc.mods.is_empty() && (bc.mods.iter().any(|m| *m == login) || login == bc.channel) {
        match parse_whisper(&bc, &login, message_text) {
            Whisper::Add(tag, webhook) => {
                let new_tag = Tag {
                    tag: tag.clone(),
                    webhook,
                };
                bc.tags.push(new_tag);
                write_config_logged(&config_file, &bc);
                info!("Tag added: {}", &tag);
                return Some((
                    bc.channel.clone(),
                    login.clone(),
                    format!("Tag added: {}", tag),
                ));
            }
            Whisper::Remove(tag) => {
                let tmp_tag = tag.clone();
                if let Some(pos) = bc.tags.iter().position(|x| *x.tag == tmp_tag) {
                    bc.tags.remove(pos);
                }
                write_config_logged(&config_file, &bc);
                info!("Tag removed: {}", &tag);
                return Some((
                    bc.channel.clone(),
                    login.clone(),
                    format!("Tag removed: {}", tag),
                ));
            }
            Whisper::List => {
                info!("List Tags");
                return Some((
                    bc.channel.clone(),
                    login.clone(),
                    format!("Tags: {}", join_tags(&bc.tags)),
                ));
            }
            Whisper::Nothing => {
                info!("Whisper ignored");
                return None;
            }
        }
    }
    None
}

async fn handle_message(
    ircclient: &Arc<IRCClient>,
    config_file: &str,
    message: twitch_irc::message::ServerMessage,
    irc_bc: &Arc<RwLock<BotConfig>>,
    activated: &mut bool,
) {
    match message {
        twitch_irc::message::ServerMessage::Privmsg(twitch_irc::message::PrivmsgMessage {
            message_text,
            message_id,
            sender,
            badges,
            ..
        }) => {
            if sender_is_ignored(irc_bc, &sender.login) {
                return;
            } else if message_text.to_lowercase() == "#deactivate" && is_mod(&badges) {
                info!("deactivated");
                log_on_discord(irc_bc, "deactivated");
                *activated = false;
            } else if message_text.to_lowercase() == "#activate" && is_mod(&badges) {
                *activated = true;
                log_on_discord(irc_bc, "activated");
                info!("activated");
            } else if *activated {
                send_messages(irc_bc, message_text, &sender, ircclient, message_id).await;
            }
        }
        twitch_irc::message::ServerMessage::Whisper(twitch_irc::message::WhisperMessage {
            sender: twitch_irc::message::TwitchUserBasics { login, .. },
            message_text,
            ..
        }) => {
            if let Some((c, u, m)) = handle_whisper(irc_bc, login, message_text, config_file) {
                log_on_discord(irc_bc, &m);
                privmsg(c, &ircclient, format!("/w {} \"{}\"", u, m)).await;
            }
        }
        _ => (),
    }
}

fn log_on_discord(irc_bc: &Arc<RwLock<BotConfig>>, message: &str) {
    let bc = irc_bc.write().unwrap();
    if !bc.log_webhook.is_empty() {
        send_message(&bc.log_webhook, "Askbot".to_string(), message.to_string());
    }
}

#[tokio::main]
pub async fn main() -> Result<(), std::io::Error> {
    env_logger::init();

    let args = std::env::args().collect::<Vec<_>>();
    let mut config_file = "config.json".to_string();
    if args.len() == 2 {
        if args[1].to_lowercase() == "generate" {
            return generate::generate();
        } else {
            config_file = args[1].to_string();
        }
    }
    info!("Use config file: {:#?}", config_file);

    match read_config(&config_file) {
        Ok(bc) => {
            let main_bc = Arc::new(RwLock::new(bc));
            #[cfg(feature = "webfrontend")]
            let rocket_bc = Arc::clone(&main_bc);
            let irc_bc = Arc::clone(&main_bc);

            let config_file2 = config_file.clone();
            #[cfg(not(feature = "webfrontend"))]
            let rocket_handle: Option<tokio::task::JoinHandle<()>> = None;
            #[cfg(feature = "webfrontend")]
            let rocket_handle = if !main_bc.read().unwrap().key.is_empty() {
                info!("start webfrontend");
                Some(tokio::spawn(async move {
                    web::rocket_main(rocket_bc, config_file2.to_string());
                }))
            } else {
                None
            };

            let config = ClientConfig::new_simple(StaticLoginCredentials::new(
                main_bc.read().unwrap().username.clone(),
                Some(main_bc.read().unwrap().oauth_token.clone()),
            ));

            let mut activated = true;

            let (mut incoming_messages, ircclient) = IRCClient::new(config);

            let irc_client = Arc::new(ircclient);
            let irc_client_main = Arc::clone(&irc_client);

            let join_handle = tokio::spawn(async move {
                while let Some(message) = incoming_messages.recv().await {
                    handle_message(&irc_client, &config_file, message, &irc_bc, &mut activated)
                        .await;
                }
            });

            let channel = main_bc.read().unwrap().channel.clone();
            irc_client_main.join(channel.clone());

            if let Some(handle) = rocket_handle {
                tokio::join![join_handle, handle];
            } else {
                tokio::join![join_handle];
            }
        }
        Err(e) => error!("Error: {}", e),
    }
    Ok(())
}
