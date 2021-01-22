#![cfg_attr(feature = "webfrontend", feature(proc_macro_hygiene, decl_macro))]

use serde::{Deserialize, Serialize};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;
use twitch_irc::TCPTransport;
use twitch_irc::TwitchIRCClient;

use std::sync::{Arc, RwLock};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

#[cfg(feature = "webfrontend")]
#[macro_use]
extern crate rocket;

#[cfg(feature = "webfrontend")]
mod web;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tag {
    tag: String,
    webhook: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BotConfig {
    channel: String,
    username: String,
    oauth_token: String,
    tags: Vec<Tag>,
    #[serde(default)]
    key: String,
    #[serde(default)]
    mods: Vec<String>,
    #[serde(default)]
    log_webhook: String,
}

async fn privmsg<T>(
    channel: String,
    client: &twitch_irc::TwitchIRCClient<TCPTransport, StaticLoginCredentials>,
    msg: T,
) where
    T: Into<String>,
{
    let r = client.privmsg(channel, msg.into()).await;
}

pub fn read_config(config_file: &str) -> std::result::Result<BotConfig, serde_any::Error> {
    serde_any::from_file(config_file)
}

pub fn write_config(
    config_file: &str,
    bc: &BotConfig,
) -> std::result::Result<(), serde_any::Error> {
    serde_any::to_file_pretty(config_file, bc)
}

fn is_mod(badges: &[twitch_irc::message::Badge]) -> bool {
    badges
        .iter()
        .any(|b| b.name == "moderator" || b.name == "broadcaster")
}
fn send_message(webhook: &str, sender: String, text: String) {
    let client = reqwest::blocking::Client::new();
    match client.post(webhook).json(&msg(sender, text)).send() {
        Ok(resp) => {
            if resp.status().is_success() {
                // TODO: nothing?
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
}

fn send_messages(
    irc_bc: &BotConfig,
    message_text: String,
    sender: &twitch_irc::message::TwitchUserBasics,
) {
    for t in irc_bc.tags.iter() {
        if message_text.to_lowercase().contains(&t.tag) {
            send_message(&t.webhook, sender.login.clone(), message_text.clone());
        }
    }
}

enum Whisper {
    Add(String, String),
    Remove(String),
    List,
    Nothing,
}

fn parse_whisper(bc: &BotConfig, login: &String, message_text: String) -> Whisper {
    if bc.mods.iter().any(|m| *m == *login) || *login == bc.channel {
        match message_text.split_whitespace().collect::<Vec<&str>>()[..] {
            ["#list"] => Whisper::List,
            ["#add", tag, webhook] => Whisper::Add(tag.to_string(), webhook.to_string()),
            ["#remove", tag] => Whisper::Remove(tag.to_string()),
            _ => Whisper::Nothing,
        }
    } else {
        return Whisper::Nothing;
    }
}

fn join_tags(ts: &[Tag]) -> String {
    ts.iter()
        .map(|t| t.tag.clone())
        .collect::<Vec<String>>()
        .join(", ")
}

async fn handle_message(
    ircclient: &Arc<TwitchIRCClient<TCPTransport, StaticLoginCredentials>>,
    config_file: &String,
    message: twitch_irc::message::ServerMessage,
    irc_bc: &Arc<RwLock<BotConfig>>,
    activated: &mut bool,
) {
    match message {
        twitch_irc::message::ServerMessage::Privmsg(twitch_irc::message::PrivmsgMessage {
            message_text,
            sender,
            badges,
            ..
        }) => {
            if message_text.to_lowercase() == "#deactivate" && is_mod(&badges) {
                info!("deactivated");
                let mut bc = irc_bc.write().unwrap();
                if !bc.log_webhook.is_empty() {
                    send_message(
                        &bc.log_webhook,
                        "Askbot".to_string(),
                        "deactivated".to_string(),
                    );
                }
                *activated = false;
            } else if message_text.to_lowercase() == "#activate" && is_mod(&badges) {
                *activated = true;
                let mut bc = irc_bc.write().unwrap();
                if !bc.log_webhook.is_empty() {
                    send_message(
                        &bc.log_webhook,
                        "Askbot".to_string(),
                        "activated".to_string(),
                    );
                }
                info!("activated");
            } else if *activated {
                send_messages(&irc_bc.read().unwrap(), message_text, &sender);
            }
        }
        twitch_irc::message::ServerMessage::Whisper(twitch_irc::message::WhisperMessage {
            sender: twitch_irc::message::TwitchUserBasics { login, .. },
            message_text,
            ..
        }) => {
            let mut reply = None;
            {
                let mut bc = irc_bc.write().unwrap();
                if !bc.mods.is_empty()
                    && (bc.mods.iter().any(|m| *m == login) || login == bc.channel)
                {
                    match parse_whisper(&bc, &login, message_text) {
                        Whisper::Add(tag, webhook) => {
                            let new_tag = Tag {
                                tag: tag.clone(),
                                webhook,
                            };
                            bc.tags.push(new_tag);
                            write_config(&config_file, &bc);
                            reply = Some((
                                bc.channel.clone(),
                                login.clone(),
                                format!("Tag added: {}", &tag),
                            ));
                            info!("Tag added: {}", tag);
                        }
                        Whisper::Remove(tag) => {
                            let tmp_tag = tag.clone();
                            if let Some(pos) = bc.tags.iter().position(|x| *x.tag == tmp_tag) {
                                bc.tags.remove(pos);
                            }
                            write_config(&config_file, &bc);
                            reply = Some((
                                bc.channel.clone(),
                                login.clone(),
                                format!("Tag removed: {}", &tag),
                            ));
                            info!("Tag removed: {}", tag);
                        }
                        Whisper::List => {
                            reply = Some((
                                bc.channel.clone(),
                                login.clone(),
                                format!("Tags: {}", join_tags(&bc.tags)),
                            ));
                            info!("List Tags");
                        }
                        Whisper::Nothing => info!("Whisper ignored"),
                    }
                }
            }
            if let Some((c, u, m)) = reply {
                {
                    let mut bc = irc_bc.write().unwrap();
                    if !bc.log_webhook.is_empty() {
                        send_message(&bc.log_webhook, "Askbot".to_string(), m.clone());
                    }
                }
                privmsg(c, &ircclient, format!("/w {} \"{}\"", u, m)).await;
            }
        }
        _ => (),
    }
}

#[tokio::main]
pub async fn main() -> Result<(), std::io::Error> {
    env_logger::init();

    let args = std::env::args().collect::<Vec<_>>();
    let mut config_file = "config.json".to_string();
    if args.len() == 2 {
        config_file = args[1].to_string();
    }
    info!("Use config file: {:#?}", config_file);

    match read_config(&config_file) {
        Ok(bc) => {
            let main_bc = Arc::new(RwLock::new(bc));
            #[cfg(feature = "webfrontend")]
            let rocket_bc = Arc::clone(&main_bc);
            let irc_bc = Arc::clone(&main_bc);

            #[cfg(feature = "webfrontend")]
            let config_file2 = config_file.clone();
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

            let (mut incoming_messages, ircclient) =
                TwitchIRCClient::<TCPTransport, StaticLoginCredentials>::new(config);

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

            join_handle.await.expect("");

            #[cfg(feature = "webfrontend")]
            if let Some(handle) = rocket_handle {
                println!("Wait for webfrontend");
                handle.await.expect("");
            }
        }
        Err(e) => error!("Error: {}", e),
    }
    Ok(())
}
