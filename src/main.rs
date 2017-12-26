extern crate chrono;
extern crate chrono_tz;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serenity;

mod announce;
mod puzzles;
mod discord;

use puzzles::Puzzle;

use serenity::model::*;
use serenity::prelude::{Client, Context, EventHandler};

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

        announce::announce_in_all(Puzzle::current_as_of_now());

        std::thread::spawn(move || Puzzle::every(|new| announce::announce_in_all(new)));
    }
    fn on_reaction_add(&self, _: Context, reaction: Reaction) {
        debug!("new reaction: {:?}", reaction.emoji);
        process_reaction(&reaction).unwrap_or_else(|e| {
            warn!(
                "failed to process reaction ({:?}): {}",
                reaction,
                e.display_chain()
            )
        });
        fn process_reaction(reaction: &Reaction) -> Result<()> {
            if reaction.emoji != CHECKMARK.into() {
                debug!("skipping reaction because it isn't a checkmark");
                return Ok(());
            }

            let channel_lock = match discord::guild_channel(discord::reaction_channel(&reaction)
                .chain_err(|| "failed to get reaction channel")?)
            {
                Some(channel_lock) => channel_lock,
                None => return Ok(()),
            };

            let message = discord::reaction_message(&reaction)
                .chain_err(|| "failed to get reaction message")?;

            if message.author.id != ::serenity::CACHE.read().unwrap().user.id {
                return Ok(());
            }

            if channel_lock.read().unwrap().name != "crosswords" {
                return Ok(());
            }

            let guild_id = channel_lock.read().unwrap().guild_id;

            let (_puzzle_channel_id, puzzle_channel) =
                find_puzzle_channel(
                    Puzzle::from_announcement(message),
                    guild_id.channels().chain_err(|| "failed to get channels")?,
                ).chain_err(|| "failed to find puzzle channel")?;

            discord::unhide_channel(&puzzle_channel, discord::from_user_id(reaction.user_id))
                .chain_err(|| "failed to hide channel")?;
            Ok(())
        }
    }

    fn on_reaction_remove(&self, _: Context, reaction: Reaction) {
        debug!("reaction removed: {:?}", reaction.emoji);
    }
}

command!(try_announce(_context, message) {
    process_message(&message).unwrap_or_else(|e| {
        warn!(
            "failed to process message ({:?}): {}",
            message,
            e.display_chain()
        )
    });

    fn process_message(message: &Message) -> Result<()> {
        if message.is_own() {
            return Ok(());
        }
        let channel_lock = match discord::guild_channel(message.channel_id.get().chain_err(|| "failed to get channel")?) {
            Some(c) => c,
            None => return Ok(()),
        };

        let channel = channel_lock.read().unwrap();

        if channel.name != "commands" {
            return Ok(());
        }

        announce::announce_in(Puzzle::current_as_of_now(), channel.guild_id).chain_err(|| "failed to announce")?;

        Ok(())
    }
});

quick_main!(run);

fn find_puzzle_channel<I>(puzzle: Puzzle, channels: I) -> Option<(ChannelId, GuildChannel)>
where
    I: IntoIterator<Item = (ChannelId, GuildChannel)>,
{
    let name = puzzle.to_channel_name();
    channels
        .into_iter()
        .find(|&(_channel_id, ref channel)| &channel.name == &name)
}

fn run() -> Result<()> {
    env_logger::init().chain_err(|| "failed to init logger")?;
    let mut client = {
        let token = std::env::var("DISCORD_TOKEN")
            .chain_err(|| "failed to retrieve token from environment")?;
        Client::new(&token, Handler)
    };

    client.with_framework(
        serenity::framework::standard::StandardFramework::new()
            .configure(|c| c.on_mention(true))
            .on("announce", try_announce),
    );

    info!("starting!");

    client.start().chain_err(|| "failed to start client")?;

    Ok(())
}
