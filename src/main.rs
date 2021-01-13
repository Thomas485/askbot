#![cfg_attr(feature = "webfrontend", feature(proc_macro_hygiene, decl_macro))]

use serde::{Deserialize, Serialize};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;
use twitch_irc::TCPTransport;
use twitch_irc::TwitchIRCClient;

use std::sync::{Arc, RwLock};

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

fn send_messages(
    irc_bc: &BotConfig,
    message_text: String,
    sender: &twitch_irc::message::TwitchUserBasics,
) {
    for t in irc_bc.tags.iter() {
        if message_text.to_lowercase().contains(&t.tag) {
            let client = reqwest::blocking::Client::new();
            match client
                .post(&t.webhook)
                .json(&msg(sender.login.clone(), message_text.clone()))
                .send()
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        // TODO: nothing?
                    } else {
                        println!(
                            "Error: Code: {} Reason: {:?}",
                            resp.status(),
                            resp.status().canonical_reason()
                        );
                    }
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            };
        }
    }
}

#[tokio::main]
pub async fn main() -> Result<(), std::io::Error> {
    let args = std::env::args().collect::<Vec<_>>();
    let mut config_file = "config.json".to_string();
    if args.len() == 2 {
        config_file = args[1].to_string();
    }
    println!("Use config file: {:#?}", config_file);

    match read_config(&config_file) {
        Ok(bc) => {
            let main_bc = Arc::new(RwLock::new(bc));
            #[cfg(feature = "webfrontend")]
            let rocket_bc = Arc::clone(&main_bc);
            let irc_bc = Arc::clone(&main_bc);

            #[cfg(feature = "webfrontend")]
            let rocket_handle = tokio::spawn(async move {
                web::rocket_main(rocket_bc, config_file.to_string());
            });

            let config = ClientConfig::new_simple(StaticLoginCredentials::new(
                main_bc.read().unwrap().username.clone(),
                Some(main_bc.read().unwrap().oauth_token.clone()),
            ));

            let mut activated = true;

            let (mut incoming_messages, ircclient) =
                TwitchIRCClient::<TCPTransport, StaticLoginCredentials>::new(config);

            let join_handle = tokio::spawn(async move {
                while let Some(message) = incoming_messages.recv().await {
                    if let twitch_irc::message::ServerMessage::Privmsg(
                        twitch_irc::message::PrivmsgMessage {
                            message_text,
                            sender,
                            badges,
                            ..
                        },
                    ) = message
                    {
                        if message_text.to_lowercase() == "#deactivate" && is_mod(&badges) {
                            println!("deactivated");
                            activated = false;
                        } else if message_text.to_lowercase() == "#activate" && is_mod(&badges) {
                            activated = true;
                            println!("activated");
                        } else if activated {
                            send_messages(&irc_bc.read().unwrap(), message_text, &sender);
                        }
                    }
                }
            });

            ircclient.join(main_bc.read().unwrap().channel.clone());

            join_handle.await.expect("");

            #[cfg(feature = "webfrontend")]
            rocket_handle.await.expect("");
        }
        Err(e) => println!("Error: {}", e),
    }
    Ok(())
}
