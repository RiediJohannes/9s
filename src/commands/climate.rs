use crate::localization::*;
use crate::{sources, Context, Error};
use poise::serenity_prelude::{CreateSelectMenuKind, Mention};
use poise::{serenity_prelude as serenity, CreateReply};
use serenity::CreateSelectMenuOption as MenuOption;
use sources::climate_forecast as forecast;
use sources::common::*;
use sources::nominatim;
use sources::nominatim::Place;

#[poise::command(slash_command, prefix_command, track_edits, aliases("temp"))]
pub async fn temperature(ctx: Context<'_>,
                         #[description = "Name of a place"] place: String,
) -> Result<(), Error> {
    // look up the requested place
    let geo_result = nominatim::query_place(&ctx.data().http_client, &place).await;

    // unwrap the geocoding response
    let places = match geo_result {
        Ok(place_list) => place_list,
        Err(e) => return Err(e.into())
    };

    if places.is_empty() {
        ctx.reply(localize!("place-not-found", search_term: &place)).await?;
    } else {
        // select a place from the list
        match select_place(ctx, &places).await {
            Selection::Unique(place) => {
                let response = create_temperature_response(&ctx.data().http_client, place).await?;
                ctx.reply(response).await?;
            },
            Selection::OneOfMany(place) => {
                let mut response = create_temperature_response(&ctx.data().http_client, place).await?;
                // Since this response will not be formatted as a reply to a slash command,
                // mention the user who invoked this command
                response = localize!("response-invoked-by",
                    message: response,
                    user_mention: Mention::User(ctx.author().id)
                );
                
                ctx.channel_id().say(ctx.http(), response).await?;
            },
            Selection::Aborted => {
                ctx.channel_id().say(ctx.http(), localize!("place-selection-timeout")).await?;
            },
            Selection::Failed(error) => {
                return Err(error)
            },
        }
    }

    Ok(())
}

pub enum Selection<T> {
    Unique(T),
    OneOfMany(T),
    Aborted,
    Failed(Error),
}


async fn create_temperature_response(client: &reqwest::Client, place: &Place) -> Result<String,Error> {
    let maybe_coordinates: Option<Coordinates> = place.into();

    match maybe_coordinates {
        Some(coordinates) => {
            let data = forecast::get_current_temperature(client, coordinates).await?;

            let last_updated_info = localize_raw!("last-updated", unix_time: data.epoch);
            let message = localize!("temperature-current-success",
                place: place.address_details(),
                celcius: data.temperature_2m,
                last_updated: last_updated_info
            );
            Ok(message)
        }
        None => {
            Err(Error::Unexpected {
                reason: "Place contained malformed coordinates!".to_string(),
                subject: Some(format!("Place: {:?}", place))
            })
        }
    }
}

// If first element matches the search term exactly and the second element does not, take the first one. Else, show the full list to pick from.
async fn select_place<'a>(ctx: Context<'_>, places: &'a [Place]) -> Selection<&'a Place> {
    if places.is_empty() {
        return Selection::Failed(Error::Unexpected {
            reason: "Received an empty set of place options.".to_string(),
            subject: None
        });
    }

    if places.len() == 1 {
        return Selection::Unique(&places[0]);
    }

    request_user_selection(ctx, places).await
}

async fn request_user_selection<'a>(ctx: Context<'_>, places: &'a [Place]) -> Selection<&'a Place> {
    const INTERACTION_ID: &str = "place_selection";

    let options: Vec<MenuOption> = places.iter().enumerate()
        .map(|(idx, p)| {
            let mut place_string = p.to_string();
            // discord limits the length of a menu option to 100 characters
            truncate_ellipsis(&mut place_string, 100, "...");
            MenuOption::new(place_string, idx.to_string())
        })
        .collect();

    // create select place prompt with the selection menu
    let place_selection = {
        let components = vec![
            serenity::CreateActionRow::SelectMenu(
                serenity::CreateSelectMenu::new(
                    INTERACTION_ID,
                    CreateSelectMenuKind::String { options })
                    .placeholder("Select place")
            ),
        ];

        CreateReply::default()
            .content(localize!("place-selection-which-one"))
            .components(components)
            .ephemeral(true)
            .reply(true)
    };

    if let Err(e) = ctx.send(place_selection).await {
        return Selection::Failed(e.into());
    }

    // react on the first interaction on the selection menu (with timeout)
    if let Some(interaction) = serenity::ComponentInteractionCollector::new(ctx)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(120))
        .filter(move |inter| inter.data.custom_id == INTERACTION_ID)
        .await
    {
        let selected_value = match &interaction.data.kind {
            serenity::ComponentInteractionDataKind::StringSelect { values} => &values[0],
            _ => panic!("unexpected interaction data kind"),
        };

        // acknowledge the interaction
        let _ = interaction.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge).await;
        interaction.delete_response(ctx.http()).await.ok();

        if let Ok(index) = selected_value.parse::<usize>() {
            if let Some(place) = places.get(index) {
                return Selection::OneOfMany(place);
            }
        };
    }

    // only reached if the interaction collector reaches its timeout
    Selection::Aborted
}