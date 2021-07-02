# askbot

A simple bot to collect twitch-chat messages in discord channels using (hash)tags.

## Usage

| command | description |
| ------- | ----------- |
| askbot generate | generate a new config file |
| askbot \<filename\> | run the bot using the configuration file \<filename\> |

## Commands (in chat)


| command | action |
| ------- | ------ |
| #deactivate | deactivates the bot temporarily |
| #activate | reactivate it again |

## Configuration file

You can generate an initial config file by running `askbot generate` and follow through the dialog.


| field | default | description |
| ----- | ------- | ----------- | 
| channel | -- | the channel to join |
| username | -- | the bots username |
| oauth\_token | -- | the corresponding oauth token (e. g. from https://twitchapps.com/tmi/) |
| tags | \[ \] | Specifies the mapping between the tags and the discord webhooks. |
| key | "" | The secret key/password for the web interface (default is deactivated) |
| mods | \[ \] | Accounts allowed to configure the bot via PM's |
| log\_webhook | "" | A discord webhook for mod-actions, etc. |
| response\_message\_success | "" | The message replied to the user on success. <br> It's prepended by @username resp. the reply-message. (see use\_reply)|
| response\_message\_failure | "" | This message is posted if there was any problem posting the message to discord (e. g. broken webhook urls, connectivity problems, etc.) |
| use\_reply | true | Use the response feature instead of @username for response messages.
| ignore | \[ \] | accounts to ignore in message handling (e. g. other bots) to prevent "bot ping pong" |

### Example file:

```json
{
  "channel": "…",
  "username": "…",
  "oauth_token": "…",
  "tags": [
    {
      "tag": "#firsttag",
      "webhook": "https://discord.com/api/webhooks/…"
    },
    {
      "tag": "#secondtag",
      "webhook": "https://discord.com/api/webhooks/…"
    }
  ],
  "mods": ["foo", "bar", "baz"],
  "ignore": ["moobot", "…"],
}
```
