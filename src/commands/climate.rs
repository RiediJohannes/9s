use crate::{Context, Error, sources};
use poise::{serenity_prelude as serenity, CreateReply};
use sources::geocoding;
use sources::geocoding::Place;
use sources::climate_forecast as forecast;
use poise::serenity_prelude::CreateSelectMenuKind;
use serenity::CreateSelectMenuOption as MenuOption;

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

#[poise::command(slash_command, prefix_command, track_edits, aliases("temp"))]
pub async fn temperature(ctx: Context<'_>,
    #[description = "Name of a place"] place: String
) -> Result<(), Error> {

    let geo_result = geocoding::query_place(&place).await;

    match geo_result {
        Ok(places) => {
            if places.is_empty() {
                ctx.say(format!("Could not find a matching place for `{}`", &place)).await?
            } else {
                match select_place(ctx, &places, &place).await {
                    Some(place) => {
                        let data = forecast::get_current_temperature(place).await?;
                        let msg = format!("The current temperature in `{}` is `{}°C`",
                            place.name, data.temperature_2m);
                        ctx.say(msg).await?
                    },
                    None => ctx.say("Place selection was cancelled").await?,
                }
            }
        },
        Err(e) => return Err(Box::new(e))
    };

    Ok(())
}


// If first element matches the search term exactly and the second element does not, take the first one. Else, show the full list to pick from.
async fn select_place<'a>(ctx: Context<'_>, places: &'a [Place], search_term: &str) -> Option<&'a Place> {
    if places.is_empty() {
        return None;
    }

    // vector has only one element or the first element matches exactly and the second already deviates from the search term
    if places.len() == 1 || places.get(1)?.name == search_term && places.get(2)?.name != search_term {
        return places.first();
    }

    request_user_selection(ctx, places).await
}

async fn request_user_selection<'a>(ctx: Context<'_>, places: &'a [Place]) -> Option<&'a Place> {
    const INTERACTION_ID: &str = "place_selection";

    let options: Vec<MenuOption> = places.iter().enumerate()
        .map(|(idx, p)| MenuOption::new(p.to_string(), idx.to_string()))
        .collect();

    let components = vec![
        serenity::CreateActionRow::SelectMenu(
            serenity::CreateSelectMenu::new(
                INTERACTION_ID,
                CreateSelectMenuKind::String { options })
                .placeholder("Select place")
        ),
    ];

    // send the question with the selection menu
    let reply = CreateReply::default()
        .content("Which one of these is the place you are looking for?")
        .components(components)
        .ephemeral(true);
    if (ctx.send(reply).await).is_err() {
        return None;
    }

    // react on the first interaction on the selection menu (with timeout)
    if let Some(interaction) = serenity::ComponentInteractionCollector::new(ctx.serenity_context())
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
        
        let _ = interaction.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge).await;
        if let Ok(index) = selected_value.parse::<usize>() {
            return places.get(index);
        }
    }

    None
}