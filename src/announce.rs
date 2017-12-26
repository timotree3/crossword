// TODO: Create functions:
// - fn is_announcement_channel(channel: GuildChannel) -> bool
// - fn is_announcement_message(message: Message) -> bool

use super::errors::*;
use error_chain::ChainedError;
use serenity::model::*;
use puzzles::Puzzle;
use CHECKMARK;

pub fn announce_in_all(new: Puzzle) {
    info!("broadcasting for puzzle: {}", new);
    let guilds = &::serenity::CACHE.read().unwrap().guilds;
    guilds.iter().for_each(|(guild_id, _guild)| {
        announce_in(new, *guild_id).unwrap_or_else(|e| {
            warn!(
                "failed to broadcast for guild (guild_id={}): {}",
                guild_id,
                e.display_chain()
            )
        })
    });
}

pub fn announce_in(puzzle: Puzzle, guild_id: GuildId) -> Result<()> {
    // get crosswords channel first both to avoid iterating over the new channel and to fail faster.
    let (crosswords_id, _crosswords) = guild_id
        .channels()
        .chain_err(|| "failed to get channels")?
        .into_iter()
        .find(|&(_channel_id, ref channel)| channel.name == "crosswords")
        .chain_err(|| "failed to find #crosswords")?;

    let _todays_channel =
        ::discord::create_unique_hidden_channel(&puzzle.to_channel_name(), guild_id)
            .chain_err(|| "failed to create todays hidden channel")?;

    crosswords_id
        .send_message(|m| {
            m.content(&puzzle.to_announcement())
                .reactions(Some(CHECKMARK))
        })
        .chain_err(|| "failed to send announcement message")?;

    Ok(())
}
