#![allow(dead_code)]

mod commands;
mod sources;

use std::sync::Arc;
use std::time::Duration;
use poise::{serenity_prelude as serenity, CreateReply, PrefixFrameworkOptions};
use serenity::GatewayIntents;

// User data, which is stored and accessible in all command invocations
#[derive(Debug)]
struct UserData {
    pub http_client: reqwest::Client,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, UserData, Error>;


#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("ENV_VAR 'DISCORD_TOKEN' could not be located!");
    let app_id = std::env::var("APPLICATION_ID").expect("ENV_VAR 'APPLICATION_ID' could not be located!");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let http_client = reqwest::Client::builder()
        .user_agent(app_id)
        .build()
        .expect("Failed to create HTTP client for future API requests.");

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("!".into()),
                edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(Duration::from_secs(3600)))),
                case_insensitive_commands: true,
                ..Default::default()
            },
            commands: vec![
                help(),
                commands::climate::age(),
                commands::climate::temperature(),
            ],
            on_error: |err| Box::pin(on_error(err)),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                // poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                poise::builtins::register_in_guild(ctx, &framework.options().commands,
                                                   serenity::GuildId::from(239525762003238912)).await?;
                Ok(UserData {
                    http_client,
                })
            })
        })
        .build();

    let mut discord_client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .expect("Failed to construct discord API client.");

    discord_client.start().await.unwrap();
}

/// Show an overview of all commands
#[poise::command(prefix_command, track_edits, slash_command)]
async fn help(
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

async fn on_error(error: poise::FrameworkError<'_, UserData, Error>) {
    println!("{:#?}", &error);

    match error {
        poise::FrameworkError::Command {error: e, ctx, .. } => {
            let command_error = format!("Hold up, something went wrong.\n{}", e);
            let _ = ctx.send(
                CreateReply::default()
                    .content(command_error)
                    .reply(true)
                    .ephemeral(true)
            ).await;
        },
        poise::FrameworkError::UnknownCommand {msg, ctx, .. } => {         
            let _ = msg.reply(&ctx.http, "Sorry, I don't know this command.").await;
        },
        _ => {
            let _ = poise::builtins::on_error(error).await;
        }
    }
}