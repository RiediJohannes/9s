// pub use fluent_templates as fluent;

// pub use fluent_bundle::FluentArgs;
// pub use fluent_templates::Loader;

macro_rules! localize {
    // Case 1: No arguments provided
    ( $text_id:expr ) => {{
        $crate::LOCALES.lookup(&crate::LANGUAGE, $text_id)
    }};

    // Case 2: One or more arguments provided
    // ( $text_id:expr, $( $arg_name:ident: $arg_value:expr ),* $(,)? ) => {{
    //     let args = FluentArgs::from_iter([
    //         $(
    //             (stringify!($arg_name), $arg_value.into())
    //         ),*
    //     ]);
    //     $crate::LOCALES.lookup_with_args(&crate::LANGUAGE, $text_id, &args)
    // }};
}

pub(crate) use localize;