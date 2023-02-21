#![cfg_attr(feature = "webfrontend", feature(proc_macro_hygiene, decl_macro))]

use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;
use twitch_irc::SecureTCPTransport;
use youtube_metadata::get_video_information;

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
    avatar_url: Option<String>,
    content: String,
    thread_name: Option<String>,
}

fn msg(username: String, content: String, thread_name: Option<String>) -> Msg {
    Msg {
        username,
        avatar_url: None,
        content,
        thread_name,
    }
}

fn is_channel_string(str: &str) -> bool {
    str == "channel"
}

fn channel_string() -> String {
    "channel".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Tag {
    tag: String,
    webhook: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    description: String,
    #[serde(skip_serializing_if = "is_channel_string")]
    #[serde(default = "channel_string")]
    #[serde(alias = "type")]
    channel_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct BotConfig {
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    channel: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    username: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
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
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    whisper_response: String,
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
    if let Some(reply_message) = reply_to {
        if let Err(e) = client.say_in_reply_to(&(channel,reply_message), msg.into()).await {
            error!("Error: {}", e);
        }
    } else {
        if let Err(e) = client.say(channel,msg.into()).await {
            error!("Error: {}", e);
        }
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

fn strip_title(title: &str) -> String {
    let end = title.len().min(80);
    if end < 80 {
        title[..end].to_string()
    } else {
        if let Some(pos) = title[..end].rfind(" ") {
            title[..pos].to_string() + "..."
        } else {
            title[..end].to_string() + "..."
        }
    }
}

fn find_url(text: &str) -> Option<String> {
    if let Some(pos) = text.find("http") {
        let title = text[pos..].to_string();
        if let Some(end) = title.find(" ") {
            return Some(title[..end].to_string());
        } else {
            return Some(title);
        }
    }
    None
}

async fn thread_title(text: &str) -> Option<String> {
    let url = find_url(text)?;
    if url.contains("youtube") || url.contains("youtu.be") {

        let url = url.replace("embed", "v");
        let information = get_video_information(&url).await.ok()?;
        return Some(strip_title(&format!("[Youtube] {}",&information.title)));
    }

    Some(strip_title(text))
}

async fn send_message(webhook: &str, sender: String, text: String, forum: bool) -> bool {
    let client = reqwest::Client::new();
    let thread_name = if forum {
        if let Some(title) = thread_title(&text).await {
            Some(title)
        } else {
            Some(strip_title(&text))
        }

    } else {
        None
    };
    match client
        .post(webhook)
        .json(&msg(sender, text, thread_name))
        .send()
        .await
    {
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

fn mention(m: &str) -> Option<String> {
    if let Some((a, b)) = m.split_once(" ") {
        if a.starts_with('!') && b.starts_with('@') && !b.contains(' ') {
            Some(b.to_string())
        } else {
            None
        }
    } else {
        None
    }
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
        // TODO: copy tags to avoid RwLock/await overlapping (send_message)
        let (tags, channel) = {
            let bc = irc_bc.read().unwrap();
            (bc.tags.clone(), bc.channel.clone())
        };
        for t in tags {
            let is_forum = t.channel_type == "forum";
            if t.tag.is_empty() {
                continue;
            }
            let text_lower = message_text.to_lowercase();
            let command = t.tag.replacen("#", "!", 1);
            if text_lower.contains(&(t.tag.to_lowercase() + " "))
                || text_lower.ends_with(&t.tag.to_lowercase())
                || (text_lower.starts_with(&command) && !t.description.is_empty())
            {
                if text_lower == command {
                    say_in_response(
                        channel.clone(),
                        client,
                        t.description.clone(),
                        Some(message_id.clone()),
                    )
                    .await;
                    continue;
                } else if let Some(user) = mention(&text_lower) {
                    say_in_response(
                        channel.clone(),
                        client,
                        format!("{} {}", user, t.description).to_string(),
                        None,
                    )
                    .await;
                    continue;
                }
                success = success
                    && send_message(
                        &t.webhook,
                        sender.login.clone(),
                        message_text.clone(),
                        is_forum,
                    )
                    .await;
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
                    description: "".to_string(),
                    channel_type: "channel".to_string(),
                };
                bc.tags.push(new_tag);
                write_config_logged(config_file, &bc);
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
                write_config_logged(config_file, &bc);
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
                info!("Mod-Whisper ignored");
                return None;
            }
        }
    } else if !bc.whisper_response.is_empty() {
        return Some((
            bc.channel.clone(),
            login.clone(),
            bc.whisper_response.clone(),
        ));
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
                log_on_discord(irc_bc, "deactivated").await;
                *activated = false;
            } else if message_text.to_lowercase() == "#activate" && is_mod(&badges) {
                *activated = true;
                log_on_discord(irc_bc, "activated").await;
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
                log_on_discord(irc_bc, &m).await;
                privmsg(c, ircclient, format!("/w {} \"{}\"", u, m)).await;
            }
        }
        _ => (),
    }
}

async fn log_on_discord(irc_bc: &Arc<RwLock<BotConfig>>, message: &str) {
    let log_webhook = {
        let bc = irc_bc.write().unwrap();
        bc.log_webhook.clone()
    };

    if !log_webhook.is_empty() {
        send_message(
            &log_webhook,
            "Askbot".to_string(),
            message.to_string(),
            false,
        )
        .await;
    }
}

#[cfg(feature = "webfrontend")]
fn create_default_config_file(path: &std::path::Path) -> anyhow::Result<()> {
    if !path.is_file() {
        info!("Try to create config file: {:?}", path);
        serde_any::to_file_pretty(
            path,
            &BotConfig {
                key: "askbot".to_string(),
                use_reply: true,
                ..std::default::Default::default()
            },
        )
        .map_err(|e| anyhow::anyhow!("Can't write default config file: {:?}", e))
    } else {
        Ok(())
    }
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
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

    #[cfg(feature = "webfrontend")]
    create_default_config_file(std::path::Path::new(&config_file))?;

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
            if let Err(e) = irc_client_main.join(channel.clone()) {
                error!("Error: {}", e);
            }

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
