#[allow(unused_imports)]
use log::{debug, error, info, warn};

use dialoguer::{theme::ColorfulTheme, theme::Theme, Confirm, Input};

use std::path::Path;

use crate::{write_config_logged, BotConfig, Tag};

pub fn generate() -> anyhow::Result<()> {
    let theme = ColorfulTheme::default();
    let file: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("The file to write the config to")
        .validate_with(|s: &String| -> Result<(), &str> {
            if s.ends_with(".yaml") || s.ends_with(".json") {
                Ok(())
            } else {
                Err("Needs to be a json or yaml file")
            }
        })
        .interact_text()?;
    let channel = prompt(&theme, "The channel the bot should join")?;
    let username = prompt(&theme, "The username of the bot")?;
    let oauth_token = prompt(
        &theme,
        "The corresponding oauth-token (e.g. https://twitchapps.com/tmi/",
    )?;
    let mods = prompt_mods(&theme)?;
    let log_webhook = prompt_log_webhook(&theme)?;
    let (response_message_success, response_message_failure) = prompt_response_messages(&theme)?;
    let ignore = prompt_ignore_list(&theme, &username)?;
    let tags = prompt_tags(&theme)?;
    let use_reply = prompt_boolean(
        &theme,
        "Do you want to use the reply functionality (instead of \"@username\")",
        true,
    )?;

    //#[cfg(feature = "webfrontend")]
    //if prompt_boolean("Do you want to the webfrontend to manage tags?", false)? {}

    let config = BotConfig {
        channel,
        username,
        oauth_token,
        key: "".to_string(),
        mods,
        log_webhook,
        response_message_success,
        response_message_failure,
        ignore,
        tags,
        use_reply,
    };

    info!("Generated config: {:#?}", config);

    prompt_writing(&theme, file, config)
}

fn prompt(theme: &dyn Theme, p: &str) -> Result<String, std::io::Error> {
    Input::with_theme(theme).with_prompt(p).interact_text()
}

fn prompt_or_empty(theme: &dyn Theme, p: &str) -> Result<String, std::io::Error> {
    Input::with_theme(theme)
        .with_prompt(p)
        .allow_empty(true)
        .interact_text()
}

fn prompt_webhook(theme: &dyn Theme, p: &str, empty: bool) -> Result<String, std::io::Error> {
    Input::with_theme(theme)
        .with_prompt(p)
        .validate_with(|s: &String| -> Result<(), &str> {
            if s.is_empty() || s.starts_with("https://discord.com/api/webhooks/") {
                Ok(())
            } else {
                Err("Needs to be a discord webhook")
            }
        })
        .allow_empty(empty)
        .interact_text()
}

fn prompt_list(theme: &dyn Theme, p: &str) -> Result<Vec<String>, std::io::Error> {
    let string: String = prompt(theme, &format!("Specify a comma separated list of {}", p))?;

    let list = string.split(',').map(|s| s.trim().to_string()).collect();
    Ok(list)
}

fn prompt_boolean(theme: &dyn Theme, p: &str, default: bool) -> Result<bool, std::io::Error> {
    Confirm::with_theme(theme)
        .with_prompt(p)
        .default(default)
        .interact()
}

fn prompt_mods(theme: &dyn Theme) -> Result<Vec<String>, std::io::Error> {
    if prompt_boolean(
        theme,
        "Do you want to specify mods, that can configure the bot via pm's?",
        false,
    )? {
        prompt_list(theme, "mods")
    } else {
        Ok(vec![])
    }
}

fn prompt_response_messages(theme: &dyn Theme) -> Result<(String, String), std::io::Error> {
    let mut response_message_success = String::new();
    let mut response_message_failure = String::new();
    if prompt_boolean(
        theme,
        "Do you want to activate response messages of the bot (e.g. \"@user: got it\")?",
        false,
    )? {
        response_message_success = prompt(theme, "The message on success")?;
        response_message_failure = prompt(theme, "The message on failure")?;
    }
    Ok((response_message_success, response_message_failure))
}

fn prompt_log_webhook(theme: &dyn Theme) -> Result<String, std::io::Error> {
    if prompt_boolean(
        theme,
        "Do you want to log the mod actions to a discord channel?",
        false,
    )? {
        prompt_webhook(theme, "The url", false)
    } else {
        Ok(String::new())
    }
}

fn prompt_ignore_list(theme: &dyn Theme, username: &str) -> Result<Vec<String>, std::io::Error> {
    let mut ignore: Vec<String> = vec![];
    if prompt_boolean(theme,&format!(
            "Do you want to specify accounts that are ignored when posting tags? ({}, moobot, etc.)",
            &username
        ),false)?
    {
        ignore = prompt_list(theme, "accounts")?;
    }
    Ok(ignore)
}

fn prompt_tags(theme: &dyn Theme) -> Result<Vec<Tag>, std::io::Error> {
    let mut tags: Vec<Tag> = vec![];
    if prompt_boolean(theme, "Do you want to specify some tags now?", true)? {
        loop {
            let tag = prompt_or_empty(theme, "Tag (empty to end the tags prompt)")?;
            if tag.is_empty() {
                break;
            }
            let webhook = prompt_webhook(theme, "Webhook (empty to discard the tag)", true)?;
            if !webhook.is_empty() {
                tags.push(Tag {
                    tag,
                    webhook,
                    description: "".to_string(),
                    channel_type: "channel".to_string(),
                });
            }
        }
    }
    Ok(tags)
}

fn prompt_writing(theme: &dyn Theme, file: String, config: BotConfig) -> anyhow::Result<()> {
    if prompt_boolean(
        theme,
        &format!("Write the configuration to {}", &file),
        false,
    )? {
        if !Path::new(&file).exists()
            || prompt_boolean(theme, "File already exists, overwrite?", false)?
        {
            write_config_logged(&file, &config);
        } else {
            println!("abort");
        }
    }
    Ok(())
}
