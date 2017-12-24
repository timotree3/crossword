extern crate chrono;
extern crate chrono_tz;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
extern crate serenity;

mod timings;

use timings::Puzzle;

use chrono::offset::Utc;
use serenity::model::*;
use serenity::prelude::*;

use error_chain::ChainedError;

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{}
}

use errors::*;

const CHECKMARK: char = '✅'; // '\u{2705}'

struct Handler;
impl EventHandler for Handler {
    fn on_ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        std::thread::spawn(move || broadcast_loop());
    }
    fn on_reaction_add(&self, _: Context, react: Reaction) {
        if let Err(e) = process_reaction(&react) {
            warn!(
                "failed to process reaction ({:?}): {}",
                react,
                e.display_chain()
            );
        }
        fn process_reaction(react: &Reaction) -> Result<()> {
            if react.emoji != CHECKMARK.into() {
                return Ok(());
            }

            let guild_id = {
                let guild_channel_lock = {
                    let channel = react
                        .channel_id
                        .get()
                        .chain_err(|| "failed to get channel")?;
                    if let Some(c) = guild_channel(channel) {
                        c
                    } else {
                        return Ok(());
                    }
                };
                let guild_channel = guild_channel_lock.read().unwrap();
                if guild_channel.name != "crosswords" {
                    return Ok(());
                }
                guild_channel.guild_id
            };

            let puzzle = Puzzle::current_as_of(
                react
                    .channel_id
                    .message(react.message_id)
                    .chain_err(|| "failed to find message")?
                    .timestamp,
            );

            mark_finished(guild_id, puzzle, react.user_id)
        }
    }

    // fn on_reaction_remove(&self, _: Context, react: Reaction) {}
}

fn guild_channel(c: Channel) -> Option<::std::sync::Arc<::std::sync::RwLock<GuildChannel>>> {
    match c {
        Channel::Guild(channel_lock) => Some(channel_lock),
        _ => None,
    }
}

fn mark_finished(guild_id: GuildId, puzzle: Puzzle, user_id: UserId) -> Result<()> {
    let name = channel_name(puzzle);
    let channels = guild_id
        .channels()
        .chain_err(|| "failed to retrieve channels")?;
    for (channel_id, _) in channels.iter().filter(|&(_, c)| c.name == name) {
        channel_id
            .create_permission(&PermissionOverwrite {
                allow: Permissions::READ_MESSAGES,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(user_id),
            })
            .chain_err(|| "failed to change user permissions")?;
    }
    Ok(())
}

quick_main!(run);

fn run() -> Result<()> {
    env_logger::init().chain_err(|| "failed to init logger")?;
    let mut client = {
        let token = std::env::var("DISCORD_TOKEN")
            .chain_err(|| "failed to retrieve token from environment")?;
        Client::new(&token, Handler)
    };

    info!("starting!");

    client.start().chain_err(|| "failed to start client")?;

    Ok(())
}

fn broadcast_loop() {
    loop {
        // let current = Puzzle::current_as_of(Utc::now());
        // current.wait_until_replaced();
        // let new = current.succ();
        std::thread::sleep(std::time::Duration::from_secs(2));
        let new = Puzzle::current_as_of(Utc::now());

        if let Err(e) = broadcast(new) {
            warn!(
                "error broadcasting puzzle ({:?}): {}",
                new,
                e.display_chain()
            );
        }
        break;
    }
}

fn broadcast(new: Puzzle) -> Result<()> {
    info!("broadcasting for puzzle: {}", new);
    let guilds = &serenity::CACHE.read().unwrap().guilds;
    for id in guilds.keys() {
        if let Err(e) = broadcast_guild(new, *id) {
            warn!(
                "failed to broadcast for guild (id={}): {}",
                id,
                e.display_chain()
            )
        }
    }
    Ok(())
}

fn broadcast_guild(puzzle: Puzzle, guild_id: GuildId) -> Result<()> {
    let channels = guild_id
        .channels()
        .chain_err(|| "failed to retrieve channels")?;

    // get crosswords channel first both to avoid iterating over the new channel and to fail faster.
    let crosswords = channels
        .iter()
        .filter(|&(_channel_id, channel)| channel.name == "crosswords")
        .map(|(_channel_id, channel)| channel)
        .next()
        .chain_err(|| "failed to find crosswords channel")?;

    let todays = guild_id
        .create_channel(&channel_name(puzzle), ChannelType::Text)
        .chain_err(|| "failed to create today's channel")?;

    // block the channel for everyone who hasn't clicked the checkmark. see process_checkmark().
    todays
        .create_permission(&PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::READ_MESSAGES,
            kind: PermissionOverwriteType::Role({
                let guild_lock = guild_id.find().chain_err(|| "failed to find cached guild")?;
                let guild = guild_lock.read().unwrap();
                let everyone_id = guild
                    .roles
                    .iter()
                    .filter(|&(_role_id, role)| role.position <= 0 && role.name == "@everyone")
                    .map(|(role_id, _role)| role_id)
                    .next()
                    .chain_err(|| "failed to find `@everyone` role")?;

                *everyone_id
            }),
        })
        .chain_err(|| "failed to configure today's channel")?;

    crosswords
        .send_message(|m| {
            m.content(&format!(
                "\u{200B}\
                 The mini of {} just came out! \
                 Play it online at https://nytimes.com/crosswords/game/mini or in the app.\n\
                 Once you're done, click the :white_check_mark: below \
                 so you can share your thoughts.",
                puzzle
            )).reactions(Some(CHECKMARK))
        })
        .chain_err(|| "failed to send update message")?;

    Ok(())
}

fn channel_name(puzzle: Puzzle) -> String {
    let (year, month, day) = puzzle.ymd();
    format!("{}-{}-{}", year, month, day)
}
