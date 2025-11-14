use std::collections::HashMap;

use lazy_static::lazy_static;
use quote::quote;

mod bin_deserialize;

/// For parsing games
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GameId {
    EU5,
    Test,
}
impl syn::parse::Parse for GameId {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let game: syn::LitStr = input.parse()?;
        return match game.value().as_str() {
            "eu5" => Ok(Self::EU5),
            "test" => Ok(Self::Test),
            _ => Err(syn::Error::new_spanned(game, "Unknown game.")),
        };
    }
}

#[proc_macro_derive(BinDeserialize, attributes(multiple, bin_token, no_brackets))]
pub fn derive_bin_deserialize(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    bin_deserialize::derive_bin_deserialize(stream)
}

fn get_tokens_from(path: &str) -> HashMap<String, u16> {
    let contents = std::fs::read_to_string(path).unwrap();

    return contents
        .lines()
        .map(|line| {
            let (id, text) = line.split_once(';').expect("invalid tokens file format");
            let id: u16 = id.parse().expect("invalid tokens file format");
            return (text.to_string(), id);
        })
        .collect();
}

lazy_static! {
    static ref EU5_TOKENS: HashMap<String, u16> =
        get_tokens_from("cartographer_web/resources/eu5/tokens.txt");
}

/// Outputs a literal for a `u16`, or gives a compile error if the token is not found.
#[proc_macro]
pub fn eu5_token(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let literal: syn::LitStr = syn::parse_macro_input!(stream);
    let text = literal.value();

    let Some(token) = EU5_TOKENS.get(&text) else {
        return syn::Error::new_spanned(literal, "Unknown token")
            .into_compile_error()
            .into();
    };

    return quote! { #token }.into();
}
