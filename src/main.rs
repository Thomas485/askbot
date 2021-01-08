use serde::{Deserialize, Serialize};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;
use twitch_irc::TCPTransport;
use twitch_irc::TwitchIRCClient;

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
struct Tag {
    tag: String,
    webhook: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct BotConfig {
    channel: String,
    username: String,
    oauth_token: String,
    tags: Vec<Tag>,
}

fn read_config() -> Option<BotConfig> {
    let file = std::fs::File::open("config.json").ok()?;
    let reader = std::io::BufReader::new(file);
    let r = serde_json::from_reader(reader);
    match r {
        Ok(res) => Some(res),
        Err(e) => {
            println!("config.json: {:#?}", e);
            None
        }
    }
}

#[tokio::main]
pub async fn main() {
    if let Some(bc) = read_config() {
        let tags = bc.tags;

        let config = ClientConfig::new_simple(StaticLoginCredentials::new(
            bc.username,
            Some(bc.oauth_token),
        ));

        let (mut incoming_messages, ircclient) =
            TwitchIRCClient::<TCPTransport, StaticLoginCredentials>::new(config);

        let join_handle = tokio::spawn(async move {
            while let Some(message) = incoming_messages.recv().await {
                if let twitch_irc::message::ServerMessage::Privmsg(
                    twitch_irc::message::PrivmsgMessage {
                        message_text,
                        sender,
                        ..
                    },
                ) = message
                {
                    for t in tags.iter() {
                        if message_text.clone().contains(&t.tag) {
                            let client = reqwest::blocking::Client::new();
                            let _res = client
                                .post(&t.webhook)
                                .json(&msg(sender.login.clone(), message_text.clone()))
                                .send()
                                .expect("Bot can't send to Discord.");
                        }
                    }
                }
            }
        });

        ircclient.join(bc.channel);
        join_handle.await.expect("");
    }
}
