use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::collections::HashSet;
use std::fs;
use std::path::Path as StdPath;
use syn::{parse::Parse, parse_macro_input, Expr, Lit, LitStr, Meta, Token};

// Configuration structure that can be built from attributes
#[derive(Clone, Debug)]
struct LocalizeConfig {
    locales_dir: String,
    languages: Vec<String>,
    fallback_language: String,
    skip_validation: Vec<String>,
    development_mode: bool,
    file_extension: String,
}

impl Default for LocalizeConfig {
    fn default() -> Self {
        Self {
            locales_dir: "locales".to_string(),
            languages: vec!["en".to_string()],
            fallback_language: "en".to_string(),
            skip_validation: vec![],
            development_mode: false,
            file_extension: "ftl".to_string(),
        }
    }
}

impl LocalizeConfig {
    fn from_meta(meta: &Meta) -> Result<Self, String> {
        let mut config = LocalizeConfig::default();

        match meta {
            Meta::List(list) => {
                list.parse_nested_meta(|meta| {
                    if meta.path.is_ident("locales_dir") {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Str(s) = lit {
                            config.locales_dir = s.value();
                        }
                        Ok(())
                    } else if meta.path.is_ident("languages") {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Str(s) = lit {
                            config.languages = s.value()
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .collect();
                        }
                        Ok(())
                    } else if meta.path.is_ident("fallback_language") {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Str(s) = lit {
                            config.fallback_language = s.value();
                        }
                        Ok(())
                    } else if meta.path.is_ident("skip_validation") {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Str(s) = lit {
                            config.skip_validation = s.value()
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .collect();
                        }
                        Ok(())
                    } else if meta.path.is_ident("development_mode") {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Bool(b) = lit {
                            config.development_mode = b.value;
                        }
                        Ok(())
                    } else if meta.path.is_ident("file_extension") {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Str(s) = lit {
                            config.file_extension = s.value();
                        }
                        Ok(())
                    } else {
                        Err(meta.error(format!("Unknown configuration option: {}", meta.path.get_ident().unwrap())))
                    }
                }).map_err(|e| e.to_string())?;
            }
            _ => return Err("Expected attribute list".to_string()),
        }

        Ok(config)
    }
}

// Parse a single .ftl file and extract message IDs
fn parse_fluent_file(content: &str) -> Result<HashSet<String>, String> {
    use fluent_syntax::parser::parse;
    use fluent_syntax::ast::Entry;

    let resource = parse(content)
        .map_err(|e| format!("Failed to parse fluent file: {:?}", e))?;

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
fn validate_message_exists(message_id: &str, config: &LocalizeConfig) -> Result<(), String> {
    let locales_dir = StdPath::new(&config.locales_dir);

    if !locales_dir.exists() {
        return Err(format!("Locales directory '{}' does not exist", config.locales_dir));
    }

    let mut missing_languages = Vec::new();

    for language in &config.languages {
        // Skip validation for languages in skip_validation list
        if config.skip_validation.contains(language) {
            continue;
        }

        let file_path = locales_dir.join(format!("{}.{}", language, config.file_extension));

        if !file_path.exists() {
            if config.development_mode {
                eprintln!("Warning: File {}.{} not found", language, config.file_extension);
                continue;
            } else {
                missing_languages.push(format!("File {}.{} not found", language, config.file_extension));
                continue;
            }
        }

        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read {}.{}: {}", language, config.file_extension, e))?;

        let message_ids = parse_fluent_file(&content)?;

        if !message_ids.contains(message_id) {
            if config.development_mode {
                eprintln!("Warning: Message '{}' not found in {}.{}", message_id, language, config.file_extension);
            } else {
                missing_languages.push(language.clone());
            }
        }
    }

    if !missing_languages.is_empty() && !config.development_mode {
        return Err(format!(
            "Message '{}' not found in languages: {}",
            message_id,
            missing_languages.join(", ")
        ));
    }

    Ok(())
}

// Parse macro arguments for the `localize` macro
struct LocalizeArgs {
    text_id: String,
    args: Vec<(String, Expr)>,
}

impl Parse for LocalizeArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let text_id: LitStr = input.parse()?;
        let mut args = Vec::new();

        // Parse optional arguments
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;

            // Check if we have more tokens
            if input.is_empty() {
                break;
            }

            // Parse key: value pairs
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let value: Expr = input.parse()?;

            args.push((key.to_string(), value));
        }

        Ok(LocalizeArgs {
            text_id: text_id.value(),
            args,
        })
    }
}

// Main attribute macro for configuration
#[proc_macro_attribute]
pub fn localize_config(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Meta);
    let input = parse_macro_input!(input as syn::ItemMod);

    // Parse configuration from attributes
    let config = match LocalizeConfig::from_meta(&args) {
        Ok(config) => config,
        Err(e) => {
            return syn::Error::new(
                Span::call_site(),
                format!("Invalid localize_config: {}", e)
            ).to_compile_error().into();
        }
    };

    // For now, just return the original module
    // In a more sophisticated implementation, you'd store this config
    // and make it available to the `localize!` macro calls within this module

    quote! {
        #input
    }.into()
}


// Simplified localize macro that uses default config or looks for stored config
#[proc_macro]
pub fn localize(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as LocalizeArgs);

    // Use default config for now - in a real implementation, you'd retrieve the stored config
    let config = LocalizeConfig::default();

    // Validate at compile time
    if let Err(error) = validate_message_exists(&args.text_id, &config) {
        return syn::Error::new(
            Span::call_site(),
            format!("Localization validation failed: {}", error)
        ).to_compile_error().into();
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

// Configuration-aware localize macro
#[proc_macro]
pub fn localize_with_config(input: TokenStream) -> TokenStream {
    // This would be a more sophisticated version that can access stored config
    // For now, this is the same as the basic localized macro
    localize(input)
}

// Helper macro to validate all messages in a module
#[proc_macro_attribute]
pub fn validate_messages(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Meta);
    let input = parse_macro_input!(input as syn::ItemMod);

    let config = match LocalizeConfig::from_meta(&args) {
        Ok(config) => config,
        Err(e) => {
            return syn::Error::new(
                Span::call_site(),
                format!("Invalid validate_messages config: {}", e)
            ).to_compile_error().into();
        }
    };

    // Create visitor to find all `localize!` macro calls
    struct LocalizeMacroVisitor<'a> {
        errors: Vec<syn::Error>,
        config: &'a LocalizeConfig,
    }

    impl<'a> syn::visit::Visit<'_> for LocalizeMacroVisitor<'a> {
        fn visit_macro(&mut self, mac: &syn::Macro) {
            if mac.path.is_ident("localize") {
                // Parse macro arguments
                if let Ok(args) = syn::parse2::<LocalizeArgs>(mac.tokens.clone()) {
                    if let Err(error) = validate_message_exists(&args.text_id, self.config) {
                        self.errors.push(syn::Error::new_spanned(
                            mac.tokens.clone(),
                            format!("Invalid message ID: {}", error)
                        ));
                    }
                }
            }
            syn::visit::visit_macro(self, mac);
        }
    }

    // Visit all items in the module to find localize! macros
    let mut visitor = LocalizeMacroVisitor {
        errors: Vec::new(),
        config: &config,
    };
    syn::visit::visit_item_mod(&mut visitor, &input);

    // If we found any validation errors, return them
    if !visitor.errors.is_empty() {
        let combined = visitor.errors.into_iter()
            .map(|e| e.to_compile_error())
            .collect::<Vec<_>>();
        return quote! { #(#combined)* }.into();
    }

    // Return the original module unmodified
    quote! {
        #input
    }.into()
}