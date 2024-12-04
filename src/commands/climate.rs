use crate::{Context, Error, sources};
use poise::{serenity_prelude as serenity, CreateReply};
use sources::nominatim;
use sources::nominatim::Place;
use sources::climate_forecast as forecast;
use poise::serenity_prelude::{CreateSelectMenuKind};
use serenity::CreateSelectMenuOption as MenuOption;

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

#[poise::command(slash_command, prefix_command, track_edits, aliases("temp"))]
pub async fn temperature(ctx: Context<'_>,
                         #[description = "Name of a place"] place: String,
) -> Result<(), Error> {
    // look up the requested place
    let geo_result = nominatim::query_place(&place).await;

    // unwrap the geocoding response
    let places = match geo_result {
        Ok(place_list) => place_list,
        Err(e) => return Err(e.into()) // Err(Box::new(e))
    };

    if places.is_empty() {
        ctx.reply(format!("Could not find a matching place for `{}`", &place)).await?;
    } else {
        // select a place from the list
        match select_place(ctx, &places, &place).await {
            Selection::Unique(place) => {
                let response = get_current_temperature(place).await?;
                ctx.reply(response).await?;
            },
            Selection::OneOfMany(place) => {
                let response = get_current_temperature(place).await?;
                ctx.channel_id().say(ctx.http(), response).await?; // maybe add "(invoked by @author)"?
            },
            Selection::Aborted => {
                ctx.channel_id().say(ctx.http(), "Place selection was cancelled".to_string()).await?;
            },
        }
    }

    Ok(())
}

pub enum Selection<T> {
    Aborted,
    Unique(T),
    OneOfMany(T),
}

async fn get_current_temperature(place: &Place) -> Result<String, Error> {
    let data = forecast::get_current_temperature(place.into()).await?;
    let msg = format!("The current temperature in **{}** is **`{}°C`** _(last updated: <t:{}:R>)_",
                      place.name.local, data.temperature_2m, data.epoch);
    Ok(msg)
}

// If first element matches the search term exactly and the second element does not, take the first one. Else, show the full list to pick from.
async fn select_place<'a>(ctx: Context<'_>, places: &'a [Place], search_term: &str) -> Selection<&'a Place> {
    if places.is_empty() {
        return Selection::Aborted;
    }

    // vector has only one element or only one that matches the search term exactly
    let exact_matches: Vec<&Place> = places.iter().filter(|&item| item.name.local == search_term).collect();
    if exact_matches.len() == 1 {
        return Selection::Unique(exact_matches.first().unwrap());
    }

    request_user_selection(ctx, places).await
}

async fn request_user_selection<'a>(ctx: Context<'_>, places: &'a [Place]) -> Selection<&'a Place> {
    const INTERACTION_ID: &str = "place_selection";

    let options: Vec<MenuOption> = places.iter().enumerate()
        .map(|(idx, p)| MenuOption::new(p.to_string(), idx.to_string()))
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
            .content("Which one of these is the place you are looking for?")
            .components(components)
            .ephemeral(true)
            .reply(true)
    };

    if (ctx.send(place_selection).await).is_err() {
        return Selection::Aborted;
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

    Selection::Aborted
}