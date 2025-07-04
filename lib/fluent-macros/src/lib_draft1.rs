use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::collections::{HashSet};
use std::fs;
use std::path::Path;
use syn::{Expr, LitStr, Token, parse_macro_input, punctuated::Punctuated};

// Configuration structure for the macro
#[derive(serde::Deserialize)]
struct LocalizeConfig {
    locales_dir: String,
    languages: Vec<String>,
    fallback_language: String,
}

impl Default for LocalizeConfig {
    fn default() -> Self {
        Self {
            locales_dir: "locales".to_string(),
            languages: vec!["en".to_string()],
            fallback_language: "en".to_string(),
        }
    }
}

// Load configuration from localize.toml or use defaults
fn load_config() -> LocalizeConfig {
    let config_path = Path::new("localize.toml");
    if config_path.exists() {
        let content = fs::read_to_string(config_path).expect("Failed to read localize.toml");
        toml::from_str(&content).expect("Failed to parse localize.toml")
    } else {
        LocalizeConfig::default()
    }
}

// Parse a single .ftl file and extract message IDs
fn parse_fluent_file(content: &str) -> Result<HashSet<String>, String> {
    use fluent_syntax::ast::Entry;
    use fluent_syntax::parser::parse;

    let resource = parse(content).map_err(|e| format!("Failed to parse fluent file: {:?}", e))?;

    let mut message_ids = HashSet::new();

    for entry in resource.body {
        match entry {
            Entry::Message(msg) => {
                message_ids.insert(msg.id.name.to_string());
            }
            Entry::Term(term) => {
                message_ids.insert(format!("-{}", term.id.name));
            }
            _ => {}
        }
    }

    Ok(message_ids)
}

// Validate that a message exists in all configured languages
fn validate_message_exists(message_id: &str) -> Result<(), String> {
    let config = load_config();
    let locales_dir = Path::new(&config.locales_dir);

    if !locales_dir.exists() {
        return Err(format!(
            "Locales directory '{}' does not exist",
            config.locales_dir
        ));
    }

    let mut missing_languages = Vec::new();

    for language in &config.languages {
        let file_path = locales_dir.join(format!("{}.ftl", language));

        if !file_path.exists() {
            missing_languages.push(format!("File {}.ftl not found", language));
            continue;
        }

        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read {}.ftl: {}", language, e))?;

        let message_ids = parse_fluent_file(&content)?;

        if !message_ids.contains(message_id) {
            missing_languages.push(language.clone());
        }
    }

    if !missing_languages.is_empty() {
        return Err(format!(
            "Message '{}' not found in languages: {}",
            message_id,
            missing_languages.join(", ")
        ));
    }

    Ok(())
}

// Parse macro arguments
struct LocalizeArgs {
    text_id: String,
    args: Vec<(String, Expr)>,
}

impl syn::parse::Parse for LocalizeArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let text_id: LitStr = input.parse()?;
        let mut args = Vec::new();

        // Parse optional arguments
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;

            let punctuated: Punctuated<syn::Expr, Token![,]> = Punctuated::parse_terminated(input)?;

            for expr in punctuated {
                if let syn::Expr::Assign(assign) = expr {
                    if let syn::Expr::Path(path) = &*assign.left {
                        if let Some(ident) = path.path.get_ident() {
                            args.push((ident.to_string(), *assign.right));
                        }
                    }
                }
            }
        }

        Ok(LocalizeArgs {
            text_id: text_id.value(),
            args,
        })
    }
}

#[proc_macro]
pub fn localize(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as LocalizeArgs);

    // Validate at compile time
    if let Err(error) = validate_message_exists(&args.text_id) {
        return syn::Error::new(
            Span::call_site(),
            format!("Localization validation failed: {}", error),
        )
        .to_compile_error()
        .into();
    }

    let text_id = &args.text_id;

    // Generate code based on whether arguments are provided
    if args.args.is_empty() {
        // Case 1: No arguments
        quote! {
            $crate::LOCALES.lookup(&crate::USER_LANG, #text_id)
        }
    } else {
        // Case 2: With arguments
        let arg_entries: Vec<_> = args.args.iter().map(|(name, value)| {
            quote! {
                (
                    std::borrow::Cow::from(#name),
                    fluent_templates::fluent_bundle::FluentValue::String(#value.to_string().into())
                )
            }
        }).collect();

        quote! {
            {
                let args = std::collections::HashMap::from_iter([
                    #(#arg_entries),*
                ]);
                $crate::LOCALES.lookup_with_args(&crate::USER_LANG, #text_id, &args)
            }
        }
    }
    .into()
}