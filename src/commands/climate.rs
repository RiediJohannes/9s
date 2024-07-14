use crate::{Context, Error, sources};
use poise::{serenity_prelude as serenity, CreateReply};
use sources::geocoding;
use sources::geocoding::Place;
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
            match select_place(ctx, &places, &place).await {
                Some(place) => ctx.say(&place.name).await?,
                None => ctx.say("Could not find a matching place").await?,
            };
        }
        Err(e) => {
            ctx.say(format!("Oh no, an error occurred! {}", e)).await?;
        }
    }

    Ok(())
}


// If first element matches the search term exactly and the second element does not, take the first one. Else, show the full list to pick from.
async fn select_place<'a>(ctx: Context<'_>, places: &'a [Place], search_term: &str) -> Option<&'a Place> {
    if places.is_empty() {
        return None;
    }

    let only_first_is_exact_match: bool = places.get(1)?.name == search_term && places.get(2)?.name != search_term;
    if places.len() == 1 || only_first_is_exact_match {
        return places.first();
    }

    request_user_selection(ctx, places).await
}

async fn request_user_selection<'a>(ctx: Context<'_>, places: &'a [Place]) -> Option<&'a Place> {
    const INTERACTION_ID: &str = "place_selection";

    let options: Vec<MenuOption> = places.iter().enumerate()
        .map(|(idx, p)| MenuOption::new(p.to_string(), idx.to_string())).collect();

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
        .components(components);
    ctx.send(reply).await.unwrap();

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

        return places.get(selected_value.parse::<usize>().unwrap());
    }

    None
}