use proc_macro;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{FieldsNamed, Ident, PathArguments, Type};

/// `=`
pub const ID_EQUAL: u16 = 0x0001;
/// `{`
pub const ID_OPEN_BRACKET: u16 = 0x0003;
/// `}`
pub const ID_CLOSE_BRACKET: u16 = 0x0004;
pub const ID_I32: u16 = 0x000c;
pub const ID_F32: u16 = 0x000d;
pub const ID_BOOL: u16 = 0x000e;
pub const ID_STRING_QUOTED: u16 = 0x000f;
pub const ID_U32: u16 = 0x0014;
pub const ID_STRING_UNQUOTED: u16 = 0x0017;
pub const ID_F64: u16 = 0x0167;
pub const ID_U64: u16 = 0x029c;
pub const ID_I64: u16 = 0x0317;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Quantifier {
    /// Always should have exactly one, no constraints on rust type
    Single,
    /// Always should have zero or one, rust type should be an [`Option`]
    Optional,
    /// Can have any number, rust type should be a [`Vec`]
    Multiple,
}

fn core_crate_ref() -> TokenStream {
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

/// Parses the `bin_token` attribute and figures out what game it belongs to
///
/// Returns tokens that will resolve to a `u16` (probably a macro)
fn extract_bin_token_attr(
    attr: &syn::Attribute,
    field_name: String,
) -> Result<TokenStream, syn::Error> {
    struct BinTokenParse {
        game: crate::GameId,
        token_name: Option<String>,
    }
    impl syn::parse::Parse for BinTokenParse {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let game = input.parse()?;

            if input.is_empty() {
                return Ok(BinTokenParse {
                    game,
                    token_name: None,
                });
            }
            let _: syn::Token![,] = input.parse()?;

            if input.is_empty() {
                return Ok(BinTokenParse {
                    game,
                    token_name: None,
                });
            }
            let token_name: syn::LitStr = input.parse()?;
            let token_name = token_name.value();

            if input.is_empty() {
                return Ok(BinTokenParse {
                    game,
                    token_name: Some(token_name),
                });
            }
            let _: syn::Token![,] = input.parse()?;

            if input.is_empty() {
                return Ok(BinTokenParse {
                    game,
                    token_name: Some(token_name),
                });
            } else {
                return Err(syn::Error::new(Span::call_site(), "Too many arguments"));
            }
        }
    }

    let args: BinTokenParse = attr.parse_args()?;

    let token_name = args.token_name.unwrap_or(field_name);
    return Ok(match args.game {
        crate::GameId::EU5 => quote! { ::pdx_parser_macros::eu5_token(#token_name) },
        crate::GameId::Test => match token_name.as_str() {
            "asdf" => quote! { 0x0101u16 },
            "extra_token" => quote! { 0x3412u16 },
            "true_false_maybe" => quote! { 0x1234u16 },
            _ => syn::Error::new(Span::call_site(), "Unknown token").to_compile_error(),
        },
    });
}

struct DeserNamedField {
    ident: Ident,
    ty: Type,
    quantifier: Quantifier,
    /// if `true`, we should always reference by the associated token and never the string
    use_token: Option<TokenStream>,
}
impl DeserNamedField {
    /// NOTE: `multiple` only signifies that it *could* be multiple.
    /// If it does not have the attribute, it is single.
    fn get_quantifier<'a>(ty: &'a syn::Type) -> (&'a syn::Type, Quantifier) {
        let Type::Path(path) = ty else {
            return (ty, Quantifier::Single);
        };

        let Some(last) = path.path.segments.last() else {
            return (ty, Quantifier::Single);
        };
        let potential_quantifier = match last.ident.to_string().as_str() {
            "Option" => Quantifier::Optional,
            "Vec" => Quantifier::Multiple,
            _ => return (ty, Quantifier::Single),
        };
        let PathArguments::AngleBracketed(args) = &last.arguments else {
            return (ty, Quantifier::Single);
        };

        if args.args.len() != 1 {
            // TODO: vec allocator is currently unstable but we may need to handle it
            return (ty, Quantifier::Single);
        };
        let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() else {
            return (ty, Quantifier::Single);
        };
        return (inner_ty, potential_quantifier);
    }

    pub fn new(field: syn::Field) -> Result<Self, syn::Error> {
        let Some(ident) = &field.ident else {
            return Err(syn::Error::new_spanned(
                field.ident,
                "Missing field identifier",
            ));
        };

        let (ty, mut quantifier) = Self::get_quantifier(&field.ty);

        let mut use_token = None;
        let mut is_multiple = false;
        for attr in &field.attrs {
            let Some(attr_ident) = attr.path().get_ident() else {
                return Err(syn::Error::new_spanned(attr.path(), "Unknown attribute"));
            };
            match attr_ident.to_string().as_str() {
                "multiple" => is_multiple = true,
                "bin_token" => use_token = Some(extract_bin_token_attr(attr, ident.to_string())?),
                _ => {
                    return Err(syn::Error::new_spanned(attr.path(), "Unknown attribute"));
                }
            }
        }

        if !is_multiple && matches!(quantifier, Quantifier::Multiple) {
            // is a vec (so could be multiple)
            // but it is just a single field containing a list
            quantifier = Quantifier::Single;
        }
        if is_multiple && !matches!(quantifier, Quantifier::Multiple) {
            return Err(syn::Error::new_spanned(
                field,
                "The `multiple` attribute is applied, but the type was not found to be a `Vec`.",
            ));
        }

        return Ok(DeserNamedField {
            ident: ident.clone(),
            ty: ty.clone(),
            quantifier,
            use_token,
        });
    }
}

fn derive_bin_deserialize_struct_named(
    ident: Ident,
    fields: FieldsNamed,
    no_brackets: bool,
) -> Result<proc_macro::TokenStream, syn::Error> {
    let pdx_parser_core = core_crate_ref();

    let fields = fields
        .named
        .into_iter()
        .map(DeserNamedField::new)
        .collect::<Result<Vec<_>, syn::Error>>()?;

    let define_field_captures: TokenStream = fields
        .iter()
        .map(|field| {
            let DeserNamedField {
                ident,
                ty,
                quantifier,
                ..
            } = field;
            if let Quantifier::Multiple = quantifier {
                return quote! {
                    let mut #ident: ::std::vec::Vec<#ty> = ::std::vec::Vec::new();
                };
            } else {
                return quote! {
                    let mut #ident: ::std::option::Option<#ty> = ::std::option::Option::None;
                };
            }
        })
        .collect();

    let use_token_match_cases: TokenStream = fields
        .iter()
        .map(|field| {
            let DeserNamedField {
                ident,
                ty,
                quantifier,
                use_token,
                ..
            } = field;
            let Some(use_token) = use_token else {
                return TokenStream::new();
            };

            let add_value = if let Quantifier::Multiple = *quantifier {
                quote! { #ident.push(value); }
            } else {
                quote! { #ident = ::std::option::Option::Some(value); }
            };

            return quote! {
                Some(#use_token) => {
                    stream.eat_token();
                    stream.parse_token(#ID_EQUAL)?;
                    let (value, rest) = #ty::take(stream)?;
                    stream = rest;
                    #add_value
                }
            };
        })
        .collect();

    let string_match_cases: TokenStream = fields
        .iter()
        .map(|field| {
            let DeserNamedField {
                ident,
                ty,
                quantifier,
                use_token,
                ..
            } = field;
            if let Some(_) = use_token {
                return TokenStream::new();
            }

            let add_value = if let Quantifier::Multiple = *quantifier {
                quote! { #ident.push(value); }
            } else {
                quote! { #ident = ::std::option::Option::Some(value); }
            };

            let key = ident.to_string();
            return quote! {
                #key => {
                    let (value, rest) = #ty::take(stream)?;
                    stream = rest;
                    #add_value
                }
            };
        })
        .collect();

    let normalize_single: TokenStream = fields
        .iter()
        .filter(|field| matches!(field.quantifier, Quantifier::Single))
        .map(|field| {
            let DeserNamedField { ident, .. } = field;

            return quote! {
                let #ident = #ident.ok_or(BinError::MissingExpectedField)?;
            };
        })
        .collect();

    let return_fields: TokenStream = fields
        .iter()
        .map(|field| {
            let DeserNamedField { ident, .. } = field;

            return quote! { #ident, };
        })
        .collect();

    let (handle_open_bracket, handle_eof, handle_close_bracket) = match no_brackets {
        true => (
            TokenStream::new(),
            quote! { break; },
            quote! { return Err(BinError::UnexpectedToken); },
        ),
        false => (
            quote! { stream.parse_token(#ID_OPEN_BRACKET)?; },
            quote! {
                return Err(BinError::EOF);
            },
            quote! {
                stream.eat_token();
                break;
            },
        ),
    };
    let impl_block = quote! {
        impl<'de> #pdx_parser_core::bin_deserialize::BinDeserialize<'de> for #ident {
            fn take(
                mut stream: #pdx_parser_core::bin_deserialize::BinDeserializer<'de>
            ) -> ::std::result::Result<
                (Self, #pdx_parser_core::bin_deserialize::BinDeserializer<'de>),
                #pdx_parser_core::bin_deserialize::BinError
            > {
                #handle_open_bracket
                use #pdx_parser_core::{
                    bin_deserialize::{
                        BinDeserializer,
                        BinDeserialize,
                        BinError,
                    },
                    common_deserialize::SkipValue,
                };
                use ::std::result::Result::{self, Err, Ok};
                use ::std::option::Option::{self, Some, None};

                #define_field_captures

                loop {
                    match stream.peek_token() {
                        None => {
                            #handle_eof
                        }
                        Some(#ID_EQUAL) => return Err(BinError::UnexpectedToken),
                        Some(#ID_CLOSE_BRACKET) => {
                            #handle_close_bracket
                        }
                        Some(#ID_STRING_QUOTED | #ID_STRING_UNQUOTED) => {
                            let (key, rest) = <&str>::take(stream)?;
                            stream = rest;
                            let Some(#ID_EQUAL) = stream.peek_token() else {
                                continue;
                            };
                            stream.eat_token();
                            match key {
                                #string_match_cases
                                _ => {
                                    stream = SkipValue::take(stream)?.1;
                                }
                            }
                        }
                        #use_token_match_cases
                        Some(_) => {
                            stream = SkipValue::take(stream)?.1;
                            if let Some(#ID_EQUAL) = stream.peek_token() {
                                stream.eat_token();
                                stream = SkipValue::take(stream)?.1;
                            }
                        }
                    }
                }

                #normalize_single
                return ::std::result::Result::Ok((
                    #ident {
                        #return_fields
                    },
                    stream,
                ));
            }
        }
    };
    return Ok(impl_block.into());
}

pub fn derive_bin_deserialize(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let syn::DeriveInput {
        attrs,
        vis,
        ident,
        generics,
        data,
    } = syn::parse_macro_input!(stream);

    let mut no_brackets = false;
    for attr in attrs {
        let Some(ident) = attr.path().get_ident().map(|ident| ident.to_string()) else {
            continue; // ignore other attributes, might not be ours
        };
        match ident.as_str() {
            "no_brackets" => {
                no_brackets = true;
            }
            _ => continue, // ignore other attributes, might not be ours
        }
    }

    let impl_block = match data {
        syn::Data::Struct(data_struct) => match data_struct.fields {
            syn::Fields::Named(fields_named) => {
                derive_bin_deserialize_struct_named(ident, fields_named, no_brackets)
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                return syn::Error::new_spanned(
                    fields_unnamed,
                    "Tuple structs are not currently supported",
                )
                .into_compile_error()
                .into();
            }
            syn::Fields::Unit => {
                return syn::Error::new(
                    Span::call_site(),
                    "Unit structs are not currently supported",
                )
                .into_compile_error()
                .into();
            }
        },
        syn::Data::Enum(_) => {
            return syn::Error::new(Span::call_site(), "Enums are not currently supported")
                .into_compile_error()
                .into();
        }
        syn::Data::Union(_) => {
            return syn::Error::new(Span::call_site(), "Unions are not currently supported")
                .into_compile_error()
                .into();
        }
    };

    return match impl_block {
        Ok(impl_block) => proc_macro::TokenStream::from(impl_block),
        Err(err) => err.into_compile_error().into(),
    };
}
