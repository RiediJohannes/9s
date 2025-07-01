use crate::localization::*;
use crate::serenity;
use crate::{Context, Error};

/// Show an overview of all commands
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<(), Error> {

    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: &localize!("help-footer"),
        ..Default::default()
    };

    poise::builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}


/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command, hide_in_help)]
pub async fn age(ctx: Context<'_>,
                 #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {

    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let discord_timestamp = localize_raw!("age-timestamp",
        unix_time: u.created_at().unix_timestamp()
    );
    let response = localize!("age-account-created-at",
        username: u.display_name(),
        timestamp: discord_timestamp
    );

    ctx.reply(response).await?;
    Ok(())
}