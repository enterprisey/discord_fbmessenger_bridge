use std::sync::mpsc;
use std::thread;

use config;
use regex::Regex;
use serde::Deserialize;

pub const MESSENGER_DELAY: u16 = 100;

fn main() {
    let mut config = config::Config::default();
    config
        .merge(config::File::with_name("settings")).expect("need a file called settings.toml")
        .merge(config::Environment::with_prefix("APP")).unwrap();
    let config = config.try_into::<Config>().expect("error reading in settings");

    let (tx, rx) = mpsc::channel(); // communication from discord to messenger
    // (messenger to discord communication happens over a websocket)
    thread::spawn(move || messenger_send::go(&rx));
    thread::spawn(move || discord::go(tx, config)).join().expect("error while calling join on discord worker");
}

#[derive(Deserialize, Debug)]
pub struct Config {

    /// The name of the discord server to connect to.
    discord_server: String,

    /// The name of the discord channel to connect to. Make sure that the webhook is also connected
    /// to this channel.
    discord_channel: String,

    /// The webhook ID to connect to. Make sure that this webhook is connected to the specified
    /// channel and server.
    discord_webhook_id: u64,

    /// The Discord token to use. Found by going to https://discordapp.com/developers/applications,
    /// selecting the application you'll use for the bridge, then selecting "Bot" from the sidebar,
    /// then generating a token.
    discord_token: String,

    /// The websocket hostname to connect to. If you're running everything on the same computer,
    /// this should be "127.0.0.1" or "::1" or "localhost". If you don't know what this means,
    /// ignore it.
    #[serde(default = "default_messenger_host")]
    messenger_host: String,

    /// The websocket port number to connect to. Make sure this is the same port number that's
    /// specified in the JavaScript code.
    messenger_port: u16,
}

fn default_messenger_host() -> String { "127.0.0.1".to_string() }

// mostly discord but also has the websocket listener
mod discord {
    use std::sync::{Arc, mpsc, Mutex};
    use std::thread;

    use lazy_static::lazy_static;
    use serenity::client::Client;
    use serenity::model::channel::Message;
    use serenity::prelude::{EventHandler, Context};
    use serenity::model::{
        id::WebhookId,
        guild::Guild,
    };

    use ws::listen;

    use super::*;

    lazy_static! {
        static ref DISCORD_MENTION: Regex = Regex::new(r"<@!\d+>").unwrap();
    }

    struct Handler {
        tx: Arc<Mutex<mpsc::Sender<String>>>,
        config: Config,
    }

    impl EventHandler for Handler {
        /*
        fn ready(&self, _ctx: Context, data: Ready) {
            // *self.current_user_id.lock().unwrap() = Some(data.user.id);
            let http = ctx.http.clone();

            // Find the #general channel
            let general_channel = data.guilds.iter().find_map(|guild| match guild {
                GuildStatus::OnlinePartialGuild(pg) => if dbg!(&pg.name) == DISCORD_SERVER {
                    Some(pg.channels(http.clone()).expect(&format!("Couldn't get channels for {} guild", DISCORD_SERVER)).keys().cloned().collect::<Vec<_>>())
                } else { None },
                GuildStatus::OnlineGuild(guild) => if dbg!(&guild.name) == DISCORD_SERVER {
                    Some(guild.channels.keys().cloned().collect::<Vec<_>>())
                } else {
                    None
                },
                GuildStatus::Offline(id) => { println!("{:?}", id); None },
                _ => None,
            })
                .expect(&format!("Couldn't find the {} Discord server", DISCORD_SERVER))
                .into_iter().find_map(|ch_id| if ch_id.name(ctx.cache.clone()).unwrap_or("".into()) == DISCORD_CHANNEL { Some(ch_id) } else { None })
                .expect(&format!("Couldn't find the {} channel", DISCORD_CHANNEL));
            thread::spawn(move || listen(format!("{}:{}", LOCALHOST, PORT), |_| {
                |msg| {
                    general_channel.say(&http, msg).expect("error sending to discord channel");
                    Ok(())
                }
            }).expect("websocket listening error"));
        }
        */

        fn guild_create(&self, ctx: Context, guild: Guild, _is_new: bool) {
            if guild.name != self.config.discord_server {
                return;
            }

            let webhook_id: WebhookId = self.config.discord_webhook_id.into();
            let webhook = guild.webhooks(&ctx.http).expect("Couldn't get webhook list")
                .iter().find(|w| w.id == webhook_id).expect("Couldn't find webhook")
                .clone();

            println!("starting websocket listener on port {}", &self.config.messenger_port);
            let websocket_addr = format!("{}:{}", &self.config.messenger_host, &self.config.messenger_port);
            thread::spawn(move || {
                listen(websocket_addr, |_| {
                    |msg: ws::Message| {
                        let msg = msg.to_string();
                        let mut msg_iter = msg.split(':');
                        let author = msg_iter.next().unwrap();
                        let msg = msg_iter.collect::<Vec<_>>().join(":");
                        webhook.execute(&ctx.http, false, |w| {
                            w.username(author);
                            w.content(msg);
                            w
                        }).expect("error executing webhook");

                        Ok(())
                    }
                }).expect("websocket listening error")
            });
        }

        fn message(&self, context: Context, msg: Message) {
            if let Ok(chan) = msg.channel_id.to_channel(context.clone()) {
                if let Some(chan) = chan.guild() {
                    if chan.read().name == self.config.discord_channel && msg.webhook_id.is_none() {
                        let guild_id = chan.read().guild_id;
                        let nick = msg.author.nick_in(&context, guild_id).unwrap_or(msg.author.name);
                        let mut message = msg.content;
                        for mentioned_user in msg.mentions {
                            let name = mentioned_user.nick_in(&context, guild_id).unwrap_or(mentioned_user.name);
                            message = DISCORD_MENTION.replace(&message, name.as_str()).to_string();
                        }

                        // if someone's message ends up with a name, there might be the "mention
                        // popup" on the messenger.com interface; if we press enter as usual, we
                        // might select a user to be mentioned instead of just sending the
                        // message. a space will cancel the popup.
                        message += " ";

                        for each_attachment in msg.attachments {
                            message += "\n";
                            message.push_str(&each_attachment.url);
                        }
                        let sender = self.tx.lock().expect("Couldn't lock discord's MPSC sender");
                        sender.send(format!("*{}*: {}", nick, message)).expect("Couldn't send message");
                    }
                }
            }
        }
    }

    pub fn go(tx: mpsc::Sender<String>, config: Config) {
        let discord_token = config.discord_token.clone();
        let handler = Handler { tx: Arc::new(Mutex::new(tx)), config };
        Client::new(discord_token, handler)
            .expect("Error creating client")
            .start().expect("error running discord client");
    }
}

// Sends messages to Messenger by literally typing them with xdotool.
mod messenger_send {
    use std::process::Command;
    use std::sync::mpsc;

    use super::*;

    pub fn go(rx: &mpsc::Receiver<String>) {
        let cmd_output = Command::new("xdotool")
            .args(&["search", "--name", "Messenger"])
            .output()
            .expect("Error running command to find the messenger window (`xdotool search --name Messenger`)");
        let window_id = std::str::from_utf8(&cmd_output.stdout).unwrap()
            .split('\n')
            .next()
            .unwrap();
        if window_id.is_empty() {
            panic!("Couldn't find a Messenger window");
        }

        println!("Found Messenger window_id = '{}'", window_id);
        while let Ok(msg) = rx.recv() {
            send_msg(msg.as_str(), window_id);
        }
    }

    pub fn send_msg(msg: &str, window_id: &str) {
        Command::new("xdotool")
            .args(&["type",
                "--window", &window_id,
                "--delay", &format!("{}", MESSENGER_DELAY),
                msg])
            .output()
            .unwrap();

        Command::new("xdotool")
            .args(&["key",
                "--window", &window_id,
                "Return"])
            .output()
            .unwrap();
    }
}
