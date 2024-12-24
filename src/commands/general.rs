use crate::serenity;
use crate::{Context, Error};

/// Show an overview of all commands
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<(), Error> {

    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: "\
Type ?help command for more info on a command.
You can edit your message to the bot and the bot will edit its response.",
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
    let response = format!("{}'s account was created at {}", u.name, u.created_at());

    ctx.reply(response).await?;
    Ok(())
}