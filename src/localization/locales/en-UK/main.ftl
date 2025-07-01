# error messages
unknown-command = Sorry, I don't know this command.
unknown-error = Hold up, something went wrong.

help-footer =
    Type '!help command' for more info on a specific command.
    You can edit your message to the bot and the bot will edit its response.

age-timestamp = <t:{ $unix_time }:f>
age-account-created-at = { $username }'s account was created at { $timestamp }

place-selection-timeout = Place selection has timed out
place-selection-placeholder = Select place
place-selection-which-one = Which one of these is the place you are looking for?
place-not-found = Could not find a matching place for `{ $search_term }`

last-updated = last updated: <t:{ $unix_time }:R>
temperature-current-success = The current temperature in **{ $place }** is **`{ $celcius }°C`** _({ $last_updated })_

# backslash before '-' is needed to escape the minus, otherwise discord sees it as an <ul>
response-invoked-by =
    { $message }
    \- invoked by { $user_mention}

