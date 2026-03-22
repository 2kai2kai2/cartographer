use crate::common::*;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{FieldsNamed, GenericArgument, Ident, PathArguments, Type, Variant};

struct DeserNamedField {
    name: Option<String>,
    ident: Ident,
    ty: Type,
    quantifier: Quantifier,
    default: Option<syn::Expr>,
    /// If set, all unknown KV pairs will be included in it.
    /// - `Vec<(K, Operator, V)>` -> (true, K, V)
    /// - `HashMap<K, (Operator, V)>` -> (false, K, V)
    /// where the `Operator` is `pdx_parser_core::common_deserialize::Operator`.
    other_keys: Option<(bool, Type, Type)>,
}
impl DeserNamedField {
    fn get_quantifier(
        ty: &syn::Type,
        is_multiple: bool,
    ) -> Result<(&syn::Type, Quantifier), syn::Error> {
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
                ));
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
        Ok((inner_ty, quantifier))
    }

    pub fn new(field: syn::Field) -> Result<Self, syn::Error> {
        let Some(ident) = &field.ident else {
            return Err(syn::Error::new_spanned(
                field.ident,
                "Missing field identifier",
            ));
        };

        let mut name = None;
        let mut is_multiple = false;
        let mut default = None;
        let mut other_keys = false;
        for attr in &field.attrs {
            let Some(attr_ident) = attr.path().get_ident() else {
                continue; // not our attribute
            };
            match attr_ident.to_string().as_str() {
                "name" => {
                    name = Some(attr.parse_args::<syn::LitStr>()?.value());
                }
                "multiple" => is_multiple = true,
                "default" => {
                    default = Some(attr.parse_args::<syn::Expr>()?);
                }
                "other_keys" => other_keys = true,
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

        // validate other_keys
        if other_keys && quantifier != Quantifier::Single {
            return Err(syn::Error::new_spanned(
                field,
                "The `other_keys` attribute is applied, but the type was not found to be a single field.",
            ));
        }
        let other_keys = if other_keys {
            if let Type::Path(tp) = &ty
                && tp.qself.is_none()
                && tp.path.segments.len() == 1
                && tp.path.segments[0].ident == "HashMap"
                && let PathArguments::AngleBracketed(args) = &tp.path.segments[0].arguments
                && args.args.len() == 2
                && let GenericArgument::Type(k) = &args.args[0]
                && let GenericArgument::Type(Type::Tuple(v)) = &args.args[1]
                && v.elems.len() == 2
                && let Type::Path(op) = &v.elems[0]
                && op.qself.is_none()
                && op.path.is_ident("Operator")
            {
                Some((false, k.clone(), v.elems[1].clone()))
            } else if let Type::Path(tp) = &ty
                && tp.qself.is_none()
                && tp.path.segments.len() == 1
                && tp.path.segments[0].ident == "Vec"
                && let PathArguments::AngleBracketed(args) = &tp.path.segments[0].arguments
                && args.args.len() == 1
                && let GenericArgument::Type(Type::Tuple(args)) = &args.args[0]
                && args.elems.len() == 3
                && let Type::Path(op) = &args.elems[1]
                && op.qself.is_none()
                && op.path.is_ident("Operator")
            {
                Some((true, args.elems[0].clone(), args.elems[2].clone()))
            } else {
                return Err(syn::Error::new_spanned(
                    field,
                    "The `other_keys` attribute is applied, but the type was not a `HashMap`.",
                ));
            }
        } else {
            None
        };

        Ok(DeserNamedField {
            name,
            ident: ident.clone(),
            ty: ty.clone(),
            quantifier,
            default,
            other_keys,
        })
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

    let other_keys = fields.iter().find(|field| field.other_keys.is_some());
    if other_keys.is_some()
        && fields
            .iter()
            .filter(|field| field.other_keys.is_some())
            .count()
            > 1
    {
        return Err(syn::Error::new(
            Span::call_site(),
            "Only one field can have the `other_keys` attribute.",
        ));
    }

    let define_field_captures = fields.iter().map(|field| {
        let DeserNamedField {
            ident,
            ty,
            quantifier,
            other_keys,
            ..
        } = field;
        if let Quantifier::Multiple = quantifier {
            quote! {
                let mut #ident: ::std::vec::Vec<#ty> = ::std::vec::Vec::new();
            }
        } else if let Some((is_vec, k, v)) = other_keys {
            if *is_vec {
                quote! {
                    let mut #ident: ::std::vec::Vec<(#k, #pdx_parser_core::common_deserialize::Operator, #v)> =
                        ::std::vec::Vec::new();
                }
            } else {
                quote! {
                    let mut #ident: ::std::collections::HashMap<
                        #k,
                        (#pdx_parser_core::common_deserialize::Operator, #v)
                    > = ::std::collections::HashMap::new();
                }
            }
        } else {
            quote! {
                let mut #ident: ::std::option::Option<#ty> = ::std::option::Option::None;
            }
        }
    });

    let match_cases = fields
        .iter()
        .filter(|field| field.other_keys.is_none())
        .map(|field| {
            let DeserNamedField {
                name,
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

            let key = name.clone().unwrap_or_else(|| ident.to_string());
            quote! {
                Some(TextToken::StringQuoted(key) | TextToken::StringUnquoted(key)) if key == #key => {
                    stream.eat_token();
                    #pdx_parser_core::Context::with_context(
                        stream.parse_token(TextToken::Equal),
                        || format!("While parsing '=' for {}", #key)
                    )?;
                    let value = #pdx_parser_core::Context::with_context(
                        stream.parse::<#ty>(),
                        || format!("While parsing value for {}", #key)
                    )?;
                    #add_value
                }
            }
        });

    let normalize_single = fields
        .iter()
        .filter(|field| matches!(field.quantifier, Quantifier::Single) && field.other_keys.is_none())
        .map(|field| {
            let DeserNamedField { ident, default, .. } = field;

            if let Some(default) = default {
                quote! {
                    let #ident = #ident.unwrap_or(#default);
                }
            } else {
                let ident_str = ident.to_string();
                quote! {
                    let #ident = #ident.ok_or(TextError::MissingExpectedField(::std::borrow::Cow::Borrowed(#ident_str)))?;
                }
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

    // Handling of other_keys

    let handle_other_keys = if let Some(field) = other_keys {
        let Some((is_vec, k, v)) = &field.other_keys else {
            unreachable!("We just checked there is one");
        };
        let ident = &field.ident;
        let add = if *is_vec {
            quote! { #ident.push((key, __op, value)); }
        } else {
            quote! { #ident.insert(key, (__op, value)); }
        };
        quote! {
            _ => {
                let key = #pdx_parser_core::Context::context(
                    stream.parse::<#k>(),
                    "While parsing arbitrary key"
                )?;
                let Ok(__op) = stream.parse::<Operator>() else {
                    continue;
                };
                let value = #pdx_parser_core::Context::with_context(
                    stream.parse::<#v>(),
                    || format!("While parsing value for {}", key)
                )?;
                #add
            }
        }
    } else {
        quote! {
            _ => {
                let _ = stream.parse::<SkipValue>()?;
                if let Some(TextToken::Equal) = stream.peek_token() {
                    stream.eat_token();
                    let _ = stream.parse::<SkipValue>()?;
                }
            }
        }
    };

    // impl
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
                    common_deserialize::{Operator, SkipValue},
                    text_deserialize::{
                        TextDeserializer,
                        TextDeserialize,
                        TextError,
                    },
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
                        #(#match_cases)*
                        #handle_other_keys
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
    Ok(impl_block.into())
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
    out
}

/// For enums that have no fields
fn derive_text_deserialize_enum_plain(
    ident: Ident,
    generics: syn::Generics,
    variants: syn::punctuated::Punctuated<Variant, syn::token::Comma>,
) -> Result<proc_macro::TokenStream, syn::Error> {
    let pdx_parser_core = core_crate_ref();

    let match_arms = variants.into_iter().map(|variant| {
        let mut enum_key = None;
        for attr in &variant.attrs {
            let Some(attr_ident) = attr.path().get_ident() else {
                continue; // not our attribute
            };
            match attr_ident.to_string().as_str() {
                "enum_key" => {
                    let attr_value = match attr.parse_args::<syn::LitStr>() {
                        Ok(value) => value,
                        Err(err) => {
                            return err.to_compile_error();
                        }
                    };
                    enum_key = Some(attr_value.value());
                }
                _ => continue, // not our attribute
            }
        }

        let variant_ident = &variant.ident;
        let enum_key =
            enum_key.unwrap_or_else(|| pascal_case_to_snake_case(&variant_ident.to_string()));

        quote! {
            #enum_key => Ok((#ident::#variant_ident, stream)),
        }
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
    Ok(impl_block.into())
}

pub fn derive_text_deserialize(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
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

    match impl_block {
        Ok(impl_block) => impl_block,
        Err(err) => err.into_compile_error().into(),
    }
}
