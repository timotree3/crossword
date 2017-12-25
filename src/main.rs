extern crate chrono;
extern crate chrono_tz;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
extern crate serenity;

mod puzzles;
mod discord;

use puzzles::Puzzle;

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
        debug!("new reaction: {:?}", react.emoji);
        if let Err(e) = process_reaction(&react) {
            warn!(
                "failed to process reaction ({:?}): {}",
                react,
                e.display_chain()
            );
        }
        fn process_reaction(react: &Reaction) -> Result<()> {
            if react.emoji != CHECKMARK.into() {
                debug!("skipping reaction because it isn't a checkmark");
                return Ok(());
            }

            let guild_id = {
                let guild_channel_lock = {
                    let channel = react
                        .channel_id
                        .get()
                        .chain_err(|| "failed to get channel")?;
                    if let Some(c) = discord::guild_channel(channel) {
                        c
                    } else {
                        debug!("skipping reaction because it isn't in #crosswords");
                        return Ok(());
                    }
                };
                let guild_channel = guild_channel_lock.read().unwrap();
                if guild_channel.name != "crosswords" {
                    return Ok(());
                }
                guild_channel.guild_id
            };

            let puzzle = {
                let announcement = react
                    .channel_id
                    .message(react.message_id)
                    .chain_err(|| "failed to find message")?;

                Puzzle::from_announcement(announcement)
            };

            mark_finished(guild_id, puzzle, react.user_id)
        }
    }

    fn on_reaction_remove(&self, _: Context, react: Reaction) {
        debug!("reaction removed: {:?}", react.emoji);
    }
}

fn mark_finished(guild_id: GuildId, puzzle: Puzzle, user_id: UserId) -> Result<()> {
    let name = puzzle.to_channel_name();
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
    // get crosswords channel first both to avoid iterating over the new channel and to fail faster.
    let crosswords =
        discord::find_channel("crosswords", guild_id).chain_err(|| "failed to find #crosswords")?;

    let _todays_channel = discord::create_secret_channel(&puzzle.to_channel_name(), guild_id)
        .chain_err(|| "failed to create todays secret channel")?;

    crosswords
        .send_message(|m| {
            m.content(&puzzle.to_announcement())
                .reactions(Some(CHECKMARK))
        })
        .chain_err(|| "failed to send announcement message")?;

    Ok(())
}
