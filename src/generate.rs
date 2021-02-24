#[allow(unused_imports)]
use log::{debug, error, info, warn};

use std::io::Write;
use std::path::Path;

use crate::{write_config, BotConfig, Tag};

fn prompt(p: &str) -> Result<String, std::io::Error> {
    let res = prompt_empty(p)?;
    if res.is_empty() {
        return prompt(p);
    } else {
        return Ok(res);
    }
}

fn prompt_empty(p: &str) -> Result<String, std::io::Error> {
    print!("{}: ", p);
    std::io::stdout().flush()?;
    let mut res = String::new();
    std::io::stdin().read_line(&mut res)?;
    let res = res.trim();
    Ok(res.to_string())
}

fn prompt_list(p: &str) -> Result<Vec<String>, std::io::Error> {
    print!("{}: ", p);
    std::io::stdout().flush()?;
    let mut res = String::new();
    std::io::stdin().read_line(&mut res)?;
    let v = res.split(",").map(|s| s.trim().to_string()).collect();
    Ok(v)
}

fn prompt_boolean(p: &str, default: bool) -> Result<bool, std::io::Error> {
    print!("{} [yN]: ", p);
    std::io::stdout().flush()?;
    let mut res = String::new();
    std::io::stdin().read_line(&mut res)?;
    let res = res.trim();
    if res.is_empty() {
        return Ok(default);
    } else if res == "y" {
        return Ok(true);
    } else if res == "n" {
        return Ok(false);
    } else {
        return prompt_boolean(p, default);
    }
}

pub fn generate() -> Result<(), std::io::Error> {
    let file = prompt("The file to write the config to")?;
    let channel = prompt("The channel the bot should join")?;
    let username = prompt("The username of the bot")?;
    let oauth_token = prompt("The corresponding oauth-token (e.g. https://twitchapps.com/tmi/ )")?;

    let mut mods: Vec<String> = vec![];
    if prompt_boolean(
        "Do you want to specify mods, that can configure the bot via pm's?",
        false,
    )? {
        mods = prompt_list("Specify a comma separated list of mods")?;
    }

    let mut log_webhook = String::new();
    if prompt_boolean(
        "Do you want to log the mod actions to a discord channel?",
        false,
    )? {
        log_webhook = prompt("The URL")?;
    }

    let mut response_message_success = String::new();
    let mut response_message_failure = String::new();
    if prompt_boolean(
        "Do you want to activate response messages of the bot (e.g. \"@user: got it\")?",
        false,
    )? {
        response_message_success =
            prompt("The message on success (is prepended by \"@username:\")")?;
        response_message_failure =
            prompt("The message on failure (is prepended by \"@username:\")")?;
    }
    let mut ignore: Vec<String> = vec![];
    if prompt_boolean(
        format!(
        "Do you want to specify accounts that are ignored when posting tags? ({}, moobot, etc.)",
        &username

    )
        .as_str(),
        false,
    )? {
        ignore = prompt_list("Specify a comma separated list of accounts")?;
    }

    #[cfg(feature = "webfrontend")]
    if prompt_boolean("Do you want to the webfrontend to manage tags?", false)? {}

    let mut tags: Vec<Tag> = vec![];
    if prompt_boolean("Do you want to specify some tags now?", false)? {
        loop {
            let tag = prompt_empty("Tag (empty to end the tags prompt)")?;
            if tag.is_empty() {
                break;
            }
            let webhook = prompt_empty("Webhook (empty to discard the tag)")?;
            if webhook.is_empty() {
                println!("Discard Tag {}", tag);
            } else {
                tags.push(Tag { tag, webhook });
            }
        }
    }

    let config = BotConfig {
        channel,
        username,
        oauth_token,
        tags,
        key: "".to_string(),
        mods,
        log_webhook,
        response_message_success,
        response_message_failure,
        ignore,
    };

    info!("Generated config: {:#?}", config);

    if prompt_boolean(
        format!("Write the configuration to {}", &file).as_str(),
        false,
    )? {
        if !Path::new(&file).exists() || prompt_boolean("File already exists, overwrite?", false)? {
            write_config(&file, &config);
        } else {
            println!("abort");
        }
    }

    Ok(())
}
