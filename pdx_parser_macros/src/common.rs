use proc_macro2::{Span, TokenStream};
use quote::quote;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quantifier {
    /// Always should have exactly one, no constraints on rust type
    Single,
    /// Always should have zero or one, rust type should be an [`Option`]
    Optional,
    /// Can have any number, rust type should be a [`Vec`]
    Multiple,
}

pub fn core_crate_ref() -> TokenStream {
    let found =
        proc_macro_crate::crate_name("pdx_parser_core").expect("pdx_parser_core is required");
    return match found {
        proc_macro_crate::FoundCrate::Itself => quote! { crate },
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, Span::call_site());
            quote! { ::#ident }
        }
    };
}
