# askbot
A simple bot to collect twitch-chat messages in discord channels via (hash)tags.

### Usage
Just change the config.json file to your needs and start the bot:

```json
{
  "channel": "the twitch cannel to listen on",
  "username": "the twitch bot name",
  "oauth_token": "...",
  "tags": [
    {
      "tag": "#firsttag",
      "webhook": "https://discord.com/api/webhooks/..."
    },
    {
      "tag": "#secondtag",
      "webhook": "https://discord.com/api/webhooks/..."
    }
  ]
}

```
