use crate::common::*;
use proc_macro;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{FieldsNamed, Ident, PathArguments, Type, Variant};

struct DeserNamedField {
    ident: Ident,
    ty: Type,
    quantifier: Quantifier,
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

        let mut is_multiple = false;
        let mut default = None;
        for attr in &field.attrs {
            let Some(attr_ident) = attr.path().get_ident() else {
                continue; // not our attribute
            };
            match attr_ident.to_string().as_str() {
                "multiple" => is_multiple = true,
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
            default,
        });
    }
}

fn derive_text_deserialize_struct_named(
    ident: Ident,
    generics: syn::Generics,
    fields: FieldsNamed,
    no_brackets: bool,
) -> Result<proc_macro::TokenStream, syn::Error> {
    let pdx_parser_core = core_crate_ref();

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

    let match_cases = fields.iter().map(|field| {
        let DeserNamedField {
            ident,
            ty,
            quantifier,
            ..
        } = field;
        let add_value = if let Quantifier::Multiple = *quantifier {
            quote! { #ident.push(value); }
        } else {
            quote! { #ident = ::std::option::Option::Some(value); }
        };

        let key = ident.to_string();
        return quote! {
            #key => {
                let (value, rest) = <#ty>::take_text(stream)?;
                stream = rest;
                #add_value
            }
        };
    });

    let normalize_single = fields
        .iter()
        .filter(|field| matches!(field.quantifier, Quantifier::Single))
        .map(|field| {
            let DeserNamedField { ident, default, .. } = field;

            if let Some(default) = default {
                return quote! {
                    let #ident = #ident.unwrap_or(#default);
                };
            } else {
                return quote! {
                    let #ident = #ident.ok_or(TextError::MissingExpectedField)?;
                };
            }
        });

    let return_fields = fields.iter().map(|field| field.ident.clone());

    let (handle_open_bracket, handle_eof, handle_close_bracket) = match no_brackets {
        true => (
            TokenStream::new(),
            quote! { break; },
            quote! { return Err(TextError::UnexpectedToken); },
        ),
        false => (
            quote! { stream.parse_token(TextToken::OpenBracket)?; },
            quote! {
                return Err(TextError::EOF);
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
        impl #impl_generics #pdx_parser_core::text_deserialize::TextDeserialize<'de> for #ident #ty_generics #where_clause {
            fn take_text(
                mut stream: #pdx_parser_core::text_deserialize::TextDeserializer<'de>
            ) -> ::std::result::Result<
                (Self, #pdx_parser_core::text_deserialize::TextDeserializer<'de>),
                #pdx_parser_core::text_deserialize::TextError
            > {
                #handle_open_bracket
                use #pdx_parser_core::{
                    text_deserialize::{
                        TextDeserializer,
                        TextDeserialize,
                        TextError,
                    },
                    common_deserialize::SkipValue,
                    text_lexer::TextToken,
                };
                use ::std::result::Result::{self, Err, Ok};
                use ::std::option::Option::{self, Some, None};

                #(#define_field_captures)*

                loop {
                    match stream.peek_token() {
                        None => {
                            #handle_eof
                        }
                        Some(TextToken::Equal) => return Err(TextError::UnexpectedToken),
                        Some(TextToken::CloseBracket) => {
                            #handle_close_bracket
                        }
                        Some(TextToken::StringQuoted(key)) | Some(TextToken::StringUnquoted(key)) => {
                            stream.eat_token();
                            let Some(TextToken::Equal) = stream.peek_token() else {
                                continue;
                            };
                            stream.eat_token();
                            match key {
                                #(#match_cases)*
                                _ => {
                                    stream = SkipValue::take_text(stream)?.1;
                                }
                            }
                        }
                        Some(_) => {
                            stream = SkipValue::take_text(stream)?.1;
                            if let Some(TextToken::Equal) = stream.peek_token() {
                                stream.eat_token();
                                stream = SkipValue::take_text(stream)?.1;
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
fn derive_text_deserialize_enum_plain(
    ident: Ident,
    generics: syn::Generics,
    variants: syn::punctuated::Punctuated<Variant, syn::token::Comma>,
) -> Result<proc_macro::TokenStream, syn::Error> {
    let pdx_parser_core = core_crate_ref();

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
        impl #impl_generics #pdx_parser_core::text_deserialize::TextDeserialize<'de> for #ident #ty_generics #where_clause {
            fn take_text(
                mut stream: #pdx_parser_core::text_deserialize::TextDeserializer<'de>
            ) -> ::std::result::Result<
                (Self, #pdx_parser_core::text_deserialize::TextDeserializer<'de>),
                #pdx_parser_core::text_deserialize::TextError
            > {
                let text: &'de str = stream.parse()?;
                return match text {
                    #(#match_arms)*
                    _ => Err(#pdx_parser_core::text_deserialize::TextError::Custom(format!("Invalid enum value \"{text}\""))),
                };
            }
        }
    };
    return Ok(impl_block.into());
}

pub fn derive_text_deserialize(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
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
                derive_text_deserialize_struct_named(ident, generics, fields_named, no_brackets)
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
                derive_text_deserialize_enum_plain(ident, generics, data_enum.variants)
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

    return match impl_block {
        Ok(impl_block) => proc_macro::TokenStream::from(impl_block),
        Err(err) => err.into_compile_error().into(),
    };
}
