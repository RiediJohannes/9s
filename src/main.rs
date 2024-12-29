#![allow(dead_code)]

mod commands;
mod sources;

use log::*;
use poise::{serenity_prelude as serenity, CreateReply, PrefixFrameworkOptions};
use serenity::GatewayIntents;
use std::sync::Arc;
use std::time::Duration;
use codes_iso_639::part_1::LanguageCode;
use thiserror::Error;


const LANGUAGE: LanguageCode = LanguageCode::De; // sets the language for bot responses, place names etc.

type Context<'a> = poise::Context<'a, ApplicationState, Error>;

// User data, which is stored and accessible in all command invocations
#[derive(Debug)]
struct ApplicationState {
    pub http_client: reqwest::Client,
}

// custom error type used throughout the project
#[derive(Debug, Error)]
pub enum Error {
    #[error("Framework error: {0}")]
    FrameworkError(#[from] serenity::Error),

    #[error("Error in API request: {0}")]
    ApiError(#[from] sources::common::ApiError),

    #[error("Unexpected error occurred: {reason:?}\nsubject: {subject:?}")]
    Unexpected {
        reason: String,
        subject: Option<String>
    },
}


#[tokio::main]
async fn main() {
    env_logger::init();

    let token = std::env::var("DISCORD_TOKEN").expect("ENV_VAR 'DISCORD_TOKEN' could not be located!");
    let app_id = std::env::var("APPLICATION_ID").expect("ENV_VAR 'APPLICATION_ID' could not be located!");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let http_client = reqwest::Client::builder()
        .user_agent(app_id)
        .connection_verbose(std::env::var("VERBOSE_LOGGING").map(|b| b.parse::<bool>().unwrap_or(false)).unwrap_or(false))
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
                commands::general::help(),
                commands::general::age(),
                commands::climate::temperature(),
            ],
            on_error: |err| Box::pin(on_error(err)),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                register_commands(ctx, framework).await?;

                // create shared state object available in every command invocation
                Ok(ApplicationState {
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


async fn on_error(error: poise::FrameworkError<'_, ApplicationState, Error>) {
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
        // use defaults for all other error types
        _ => {
            let _ = poise::builtins::on_error(error).await;
        }
    }
}

async fn register_commands(ctx: &poise::serenity_prelude::Context, framework: &poise::Framework<ApplicationState, Error>)
    -> Result<(), Error>
{
    poise::builtins::register_globally(ctx, &framework.options().commands).await?;

    // register slash commands in every test guild for immediate access
    let guild_ids = std::env::var("TEST_GUILD_IDS")
        .map(|ids| ids
            .split(',')
            .filter_map(|id| match id.trim().parse() {
                Ok(id) => Some(id),
                Err(_) => {
                    warn!("Failed to parse test guild id: {}", id);
                    None
                },
            })
            .collect::<Vec<u64>>()
        )
        .unwrap_or_default();

    for guild in guild_ids {
        poise::builtins::register_in_guild(ctx, &framework.options().commands,
                                           serenity::GuildId::from(guild)).await?;
    }
    
    Ok(())
}