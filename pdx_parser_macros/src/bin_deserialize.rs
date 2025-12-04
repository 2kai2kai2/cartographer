use crate::common::*;
use proc_macro;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{FieldsNamed, Ident, PathArguments, Type, Variant};

/// `=`
#[allow(unused)]
pub const ID_EQUAL: u16 = 0x0001;
/// `{`
#[allow(unused)]
pub const ID_OPEN_BRACKET: u16 = 0x0003;
/// `}`
#[allow(unused)]
pub const ID_CLOSE_BRACKET: u16 = 0x0004;
#[allow(unused)]
pub const ID_I32: u16 = 0x000c;
#[allow(unused)]
pub const ID_F32: u16 = 0x000d;
#[allow(unused)]
pub const ID_BOOL: u16 = 0x000e;
#[allow(unused)]
pub const ID_STRING_QUOTED: u16 = 0x000f;
#[allow(unused)]
pub const ID_U32: u16 = 0x0014;
#[allow(unused)]
pub const ID_STRING_UNQUOTED: u16 = 0x0017;
#[allow(unused)]
pub const ID_F64: u16 = 0x0167;
#[allow(unused)]
pub const ID_U64: u16 = 0x029c;
#[allow(unused)]
pub const ID_I64: u16 = 0x0317;

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
        crate::GameId::EU5 => quote! { ::pdx_parser_macros::eu5_token!(#token_name) },
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
    default: Option<syn::Expr>,
}
impl DeserNamedField {
    fn get_quantifier<'a>(
        ty: &'a syn::Type,
        is_multiple: bool,
    ) -> Result<(&'a syn::Type, Quantifier), syn::Error> {
        let Type::Path(path) = ty else {
            return Ok((ty, Quantifier::Single));
        };

        let Some(last) = path.path.segments.last() else {
            return Ok((ty, Quantifier::Single));
        };
        let quantifier = match last.ident.to_string().as_str() {
            "Vec" if is_multiple => Quantifier::Multiple,
            _ if is_multiple => {
                return Err(syn::Error::new_spanned(
                    ty,
                    "The multiple attribute can only be applied to a Vec type",
                ))
            }
            "Option" => Quantifier::Optional,
            _ => return Ok((ty, Quantifier::Single)),
        };
        let PathArguments::AngleBracketed(args) = &last.arguments else {
            return Ok((ty, Quantifier::Single));
        };

        if args.args.len() != 1 {
            // TODO: vec allocator is currently unstable but we may need to handle it
            return Ok((ty, Quantifier::Single));
        };
        let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() else {
            return Ok((ty, Quantifier::Single));
        };
        return Ok((inner_ty, quantifier));
    }

    pub fn new(field: syn::Field) -> Result<Self, syn::Error> {
        let Some(ident) = &field.ident else {
            return Err(syn::Error::new_spanned(
                field.ident,
                "Missing field identifier",
            ));
        };

        let mut use_token = None;
        let mut is_multiple = false;
        let mut default = None;
        for attr in &field.attrs {
            let Some(attr_ident) = attr.path().get_ident() else {
                continue; // not our attribute
            };
            match attr_ident.to_string().as_str() {
                "multiple" => is_multiple = true,
                "bin_token" => use_token = Some(extract_bin_token_attr(attr, ident.to_string())?),
                "default" => {
                    default = Some(attr.parse_args::<syn::Expr>()?);
                }
                _ => continue, // not our attribute
            }
        }

        let (ty, quantifier) = Self::get_quantifier(&field.ty, is_multiple)?;
        if default.is_some() && !matches!(quantifier, Quantifier::Single) {
            return Err(syn::Error::new_spanned(
                field,
                "The `default` attribute is applied, but the type was not found to be a single field.",
            ));
        }

        return Ok(DeserNamedField {
            ident: ident.clone(),
            ty: ty.clone(),
            quantifier,
            use_token,
            default,
        });
    }
}

fn derive_bin_deserialize_struct_named(
    ident: Ident,
    generics: syn::Generics,
    fields: FieldsNamed,
    no_brackets: bool,
) -> Result<proc_macro::TokenStream, syn::Error> {
    let pdx_parser_core = core_crate_ref();
    let struct_name = ident.to_string();

    let fields = fields
        .named
        .into_iter()
        .map(DeserNamedField::new)
        .collect::<Result<Vec<_>, syn::Error>>()?;

    let define_field_captures = fields.iter().map(|field| {
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
    });

    let use_token_match_cases = fields.iter().map(|field| {
        let DeserNamedField {
            ident,
            ty,
            quantifier,
            use_token,
            ..
        } = field;
        let field_name = ident.to_string();
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
                stream.parse_token(#ID_EQUAL)
                    .map_err(|err| err.context(format!("Missing equal sign for {} in {}", #field_name, #struct_name)))?;
                let (value, rest) = <#ty>::take(stream)
                    .map_err(|err| err.context(format!("While parsing value for {} in {}", #field_name, #struct_name)))?;
                stream = rest;
                #add_value
            }
        };
    });

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
                    let (value, rest) = <#ty>::take(stream)
                        .map_err(|err| err.context(format!("While parsing value for {key} in {}", #struct_name)))?;
                    stream = rest;
                    #add_value
                }
            };
        })
        .collect();

    let normalize_single = fields
        .iter()
        .filter(|field| matches!(field.quantifier, Quantifier::Single))
        .map(|field| {
            let DeserNamedField { ident, default, .. } = field;
            let ident_str = ident.to_string();

            if let Some(default) = default {
                return quote! {
                    let #ident = #ident.unwrap_or(#default);
                };
            } else {
                return quote! {
                    let #ident = #ident.ok_or(BinError::MissingExpectedField(#ident_str.to_string()))?;
                };
            }
        });

    let return_fields = fields.iter().map(|field| field.ident.clone());

    let (handle_open_bracket, handle_eof, handle_close_bracket) = match no_brackets {
        true => (
            TokenStream::new(),
            quote! { break; },
            quote! {
                return Err(
                    BinError::UnexpectedToken(#ID_CLOSE_BRACKET)
                        .context(format!("At idx {} when we were expecting the {} object to be terminated by EOF", stream.current_index(), #struct_name))
                );
            },
        ),
        false => (
            quote! {
                stream.parse_token(#ID_OPEN_BRACKET)
                    .map_err(|err| err.context(format!("Missing open bracket at start of {} struct at idx {}", #struct_name, stream.current_index())))?;
            },
            quote! {
                return Err(BinError::EOF.context(format!("When we were expecting the {} object to be terminated by '}}'", #struct_name)));
            },
            quote! {
                stream.eat_token();
                break;
            },
        ),
    };

    let has_lifetime_de = generics.params.iter().any(|generic| match generic {
        syn::GenericParam::Lifetime(lifetime) => lifetime.lifetime.ident == "de",
        _ => false,
    });
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut _generics_add_de: syn::Generics;
    let impl_generics = if has_lifetime_de {
        impl_generics
    } else {
        _generics_add_de = generics.clone();
        _generics_add_de.params.push(syn::parse_quote! { 'de });
        _generics_add_de.split_for_impl().0
    };

    let impl_block = quote! {
        impl #impl_generics #pdx_parser_core::bin_deserialize::BinDeserialize<'de> for #ident #ty_generics #where_clause {
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

                #(#define_field_captures)*

                loop {
                    match stream.peek_token() {
                        None => {
                            #handle_eof
                        }
                        Some(#ID_EQUAL) => {
                            return Err(
                                BinError::UnexpectedToken(#ID_EQUAL)
                                    .context(format!("At idx {} when a new KV or value was expected in {}", stream.current_index(), #struct_name))
                            );
                        }
                        Some(#ID_CLOSE_BRACKET) => {
                            #handle_close_bracket
                        }
                        Some(#ID_STRING_QUOTED | #ID_STRING_UNQUOTED) => {
                            let key: &'de str = stream.parse().map_err(|err| {
                                err.context(format!("While parsing string key at idx {} in {}", stream.current_index(), #struct_name))
                            })?;
                            let Some(#ID_EQUAL) = stream.peek_token() else {
                                continue;
                            };
                            stream.eat_token();
                            match key {
                                #string_match_cases
                                _ => {
                                    let SkipValue = stream.parse().map_err(|err| {
                                        err.context(format!("While skipping value for uncaptured key {key} in {} at idx {}", #struct_name, stream.current_index()))
                                    })?;
                                }
                            }
                        }
                        #(#use_token_match_cases)*
                        Some(_) => {
                            let SkipValue = stream.parse().map_err(|err| {
                                err.context(format!("While skipping non-string key in {}", #struct_name))
                            })?;
                            if let Some(#ID_EQUAL) = stream.peek_token() {
                                stream.eat_token();
                                let SkipValue = stream.parse().map_err(|err| {
                                    err.context(format!("While skipping value for non-string key in {}", #struct_name))
                                })?;
                            }
                        }
                    }
                }

                #(#normalize_single)*
                return ::std::result::Result::Ok((
                    #ident {
                        #(#return_fields,)*
                    },
                    stream,
                ));
            }
        }
    };
    return Ok(impl_block.into());
}

fn pascal_case_to_snake_case(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_uppercase() {
            if !out.is_empty() {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    return out;
}

/// For enums that have no fields
fn derive_bin_deserialize_enum_plain(
    ident: Ident,
    generics: syn::Generics,
    variants: syn::punctuated::Punctuated<Variant, syn::token::Comma>,
) -> Result<proc_macro::TokenStream, syn::Error> {
    let pdx_parser_core = core_crate_ref();
    let enum_name = ident.to_string();

    let match_arms = variants.into_iter().map(|variant| {
        let variant_ident = &variant.ident;
        let snake_case = pascal_case_to_snake_case(&variant_ident.to_string());
        return quote! {
            #snake_case => Ok((#ident::#variant_ident, stream)),
        };
    });

    let has_lifetime_de = generics.params.iter().any(|generic| match generic {
        syn::GenericParam::Lifetime(lifetime) => lifetime.lifetime.ident == "de",
        _ => false,
    });
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut _generics_add_de: syn::Generics;
    let impl_generics = if has_lifetime_de {
        impl_generics
    } else {
        _generics_add_de = generics.clone();
        _generics_add_de.params.push(syn::parse_quote! { 'de });
        _generics_add_de.split_for_impl().0
    };

    let impl_block = quote! {
        impl #impl_generics #pdx_parser_core::bin_deserialize::BinDeserialize<'de> for #ident #ty_generics #where_clause {
            fn take(
                mut stream: #pdx_parser_core::bin_deserialize::BinDeserializer<'de>
            ) -> ::std::result::Result<
                (Self, #pdx_parser_core::bin_deserialize::BinDeserializer<'de>),
                #pdx_parser_core::bin_deserialize::BinError
            > {
                let text: &'de str = stream.parse()
                    .map_err(|err| err.context(format!("While parsing string value for enum {}", #enum_name)))?;
                return match text {
                    #(#match_arms)*
                    _ => ::std::result::Result::Err(
                        #pdx_parser_core::bin_deserialize::BinError::Custom(format!("Invalid enum value \"{text}\" for {}", #enum_name))
                    ),
                };
            }
        }
    };
    return Ok(impl_block.into());
}

pub fn derive_bin_deserialize(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let syn::DeriveInput {
        attrs,
        ident,
        generics,
        data,
        ..
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
                derive_bin_deserialize_struct_named(ident, generics, fields_named, no_brackets)
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
        syn::Data::Enum(data_enum) => {
            if data_enum
                .variants
                .iter()
                .all(|variant| variant.fields.is_empty())
            {
                derive_bin_deserialize_enum_plain(ident, generics, data_enum.variants)
            } else {
                return syn::Error::new(
                    Span::call_site(),
                    "Non-plain enums are not currently supported",
                )
                .into_compile_error()
                .into();
            }
        }
        syn::Data::Union(_) => {
            return syn::Error::new(Span::call_site(), "Unions are not currently supported")
                .into_compile_error()
                .into();
        }
    };

    let impl_block = impl_block.map(|block| -> TokenStream {
        let block: proc_macro2::TokenStream = block.into();
        return quote! {
           #[automatically_derived]
           #block
        }
        .into();
    });
    return match impl_block {
        Ok(impl_block) => proc_macro::TokenStream::from(impl_block),
        Err(err) => err.into_compile_error().into(),
    };
}
