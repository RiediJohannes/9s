#![allow(dead_code)]

mod commands;
mod sources;
mod localization;
mod utils;


use fluent_templates::{langid, LanguageIdentifier};
use lazy_static::lazy_static;
use localization::*;
use log::*;
use poise::{serenity_prelude as serenity, CreateReply, PrefixFrameworkOptions};
use serenity::GatewayIntents;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;


const USER_LANG_ENV: Option<&str> = std::option_env!("USER_LANGUAGE");
const QUERY_LANG_ENV: Option<&str> = std::option_env!("QUERY_LANGUAGE");
const FALLBACK_LANGUAGE: LanguageIdentifier = langid!("en-UK");

lazy_static! {
    static ref USER_LANG: LanguageIdentifier = USER_LANG_ENV.map_or_else(|| FALLBACK_LANGUAGE,
        |s| s.parse().expect("FATAL ERROR: Malformed fallback language"));

    static ref QUERY_LANG: LanguageIdentifier = QUERY_LANG_ENV.map_or_else(|| FALLBACK_LANGUAGE,
        |s| s.parse().expect("FATAL ERROR: Malformed fallback language"));
}


type Context<'a> = poise::Context<'a, ApplicationState, Error>;

// application-scoped data, which is stored and accessible in all command invocations
#[derive(Debug)]
struct ApplicationState {
    pub http_client: reqwest::Client,
}

// custom top-level error type used throughout the project
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

// set up text sources for fluent localizations
fluent_templates::static_loader! {
    static LOCALES = {
        // The directory of localizations and fluent resources.
        locales: "./src/localization/locales",
        fallback_language: "en-UK",
        // A fluent resource that is shared with every locale.
        // core_locales: "./locales/core.ftl",
    };
}


#[tokio::main]
async fn main() {
    env_logger::init();

    // check if querying ENV variables succeeded
    if USER_LANG_ENV.is_none() {
        warn!("Failed to parse ENV variable 'USER_LANGUAGE'. Falling back to default language '{}'", FALLBACK_LANGUAGE);
    }
    if QUERY_LANG_ENV.is_none() {
        warn!("Failed to parse ENV variable 'QUERY_LANGUAGE'. Falling back to default language '{}'", FALLBACK_LANGUAGE);
    }

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
        poise::FrameworkError::Command {ctx, .. } => {
            let _ = ctx.send(
                CreateReply::default()
                    .content(localize!("unknown-error"))
                    .reply(true)
                    .ephemeral(true)
            ).await;
        },
        poise::FrameworkError::UnknownCommand {msg, ctx, .. } => {
            let _ = msg.reply(&ctx.http, localize!("unknown-command")).await;
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