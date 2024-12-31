use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident, Token, braced};
use syn::parse::{Parse, ParseStream, Result};
use std::collections::HashMap;

struct FieldsMap {
    methods: HashMap<Ident, Vec<Ident>>
}

impl Parse for FieldsMap {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        braced!(content in input);
        let mut methods = HashMap::new();

        while !content.is_empty() {
            let method_name: Ident = content.parse()?;
            let _: Token![=] = content.parse()?;

            let fields_content;
            syn::bracketed!(fields_content in content);
            let fields = fields_content
                .parse_terminated(Ident::parse, Token![,])?
                .into_iter()
                .collect();

            methods.insert(method_name, fields);

            if !content.is_empty() {
                let _: Token![,] = content.parse()?;
            }
        }

        Ok(FieldsMap { methods })
    }
}

#[proc_macro_attribute]
pub fn collect_fields(attr: TokenStream, item: TokenStream) -> TokenStream {
    let FieldsMap { methods } = parse_macro_input!(attr as FieldsMap);
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;

    let implementations = methods.iter().map(|(method_name, fields)| {
        let field_refs = fields.iter().map(|field| {
            quote! { self.#field.as_ref() }
        });

        quote! {
            pub fn #method_name(&self) -> Option<Vec<String>> {
                let levels = [
                    #(#field_refs),*
                ];
                bot_utils::collect_somes(&levels)
            }
        }
    });

    let expanded = quote! {
        #input

        impl #struct_name {
            #(#implementations)*
        }
    };

    expanded.into()
}