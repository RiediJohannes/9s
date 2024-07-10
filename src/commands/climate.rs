use crate::{Context, Error, sources};
use poise::{serenity_prelude as serenity};
use sources::geocoding;


/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
pub async fn age(ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {

    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());

    ctx.say(response).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, track_edits)]
pub async fn temperature(ctx: Context<'_>,
    #[description = "Name of a place"] place: String
) -> Result<(), Error> {

    let geo_result = geocoding::query_place(&place).await;

    if let Ok(Some(place)) = geo_result {
        ctx.say(&place.name).await?;
    }

    Ok(())
}