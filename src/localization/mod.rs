pub use fluent_templates::Loader;

macro_rules! localize {
    // Case 1: No arguments provided
    ( $text_id:expr ) => {{
        $crate::LOCALES.lookup(&crate::LANGUAGE, $text_id)
    }};

    // Case 2: One or more arguments provided
    ( $text_id:expr, $( $arg_name:ident: $arg_value:expr ),* $(,)? ) => {{
        let args = std::collections::HashMap::from_iter([
            $(  // key-value pair (Cow<str>, FluentValue::String)
                (std::borrow::Cow::from(stringify!($arg_name)),
                fluent_templates::fluent_bundle::FluentValue::String($arg_value.to_string().into()))
            ),*
        ]);
        $crate::LOCALES.lookup_with_args(&crate::LANGUAGE, $text_id, &args)
    }};
}

macro_rules! localize_raw {
    // Case 1: No arguments provided
    ( $text_id:expr ) => {{
        let output = localize!($text_id);
        output.replace("\u{2068}", "").replace("\u{2069}", "")
    }};

    // Case 2: One or more arguments provided
    ( $text_id:expr, $( $arg_name:ident: $arg_value:expr ),* $(,)? ) => {{
        let output = localize!($text_id, $( $arg_name: $arg_value ),*);
        output.replace("\u{2068}", "").replace("\u{2069}", "")
    }};
}

pub(crate) use localize;
pub(crate) use localize_raw;