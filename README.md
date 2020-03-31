# discord_fbmessenger_bridge
A hacky little proof-of-concept bridge between Facebook Messenger and Discord

## How to use
Don't.

## no u
Setup:
1. Make sure `xdotool` is installed.
2. Set up a new [Discord webhook](https://support.discordapp.com/hc/en-us/articles/228383668-Intro-to-Webhooks). Remember the URL you get. It'll be of the form "https://discordapp.com/api/webhooks/NUMBERS/STUFF". Remember the NUMBERS part; it's the Webhook ID.
3. Copy `settings.toml.example` to `settings.toml` and customize according to the comments in that file.

Run:
1. Open a tab of Facebook Messenger, log in, and open a chat. I've tested this in Firefox; no idea if it works in Chrome.
2. Then, start the Rust half by typing `cargo run`. Fix any errors.
3. Paste the contents of `blob.js` into the browser console, hit Enter, and click in the text field where you normally type messages.

If everything worked, messages sent by other people in the Facebook chat should show up in the Discord channel, and messages sent in the Discord channel should show up in the Facebook chat.

## Other junk
No guarantees whatsoever that this'll work. Heck, I've only tried it once. Use only with [whitehat accounts](https://www.facebook.com/whitehat/accounts/) for research/educational purposes.
