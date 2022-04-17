// Copyright 2020 - developers of the `grammers` project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{parse_macro_input, AttributeArgs, Ident, ItemFn, LitStr, NestedMeta};

#[derive(PartialEq)]
pub(crate) enum UpdateType {
    Message,
    CallbackQuery,
    InlineQuery,
}

impl UpdateType {
    fn as_str(&self) -> &'static str {
        match self {
            UpdateType::Message => "Message",
            UpdateType::CallbackQuery => "Callback_Query",
            UpdateType::InlineQuery => "Inline_Query",
        }
    }
}

impl ToTokens for UpdateType {
    fn to_tokens(&self, stream: &mut TokenStream2) {
        let ident = Ident::new(self.as_str(), Span::call_site());
        stream.append(ident);
    }
}

struct Args {
    pattern: LitStr,
    is_regex: bool,
    is_command: bool,
}

impl Args {
    fn new(args: AttributeArgs) -> syn::Result<Self> {
        let mut pattern = None;
        let mut is_regex = None;
        let mut is_command = None;

        for arg in args {
            match arg {
                NestedMeta::Lit(syn::Lit::Str(lit)) => match pattern {
                    None => {
                        pattern = Some(lit);
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            lit,
                            "multiple patterns specified, expected one.",
                        ))
                    }
                },
                NestedMeta::Meta(syn::Meta::NameValue(name_value)) => {
                    if name_value.path.is_ident("pattern") {
                        if let syn::Lit::Str(lit) = name_value.lit {
                            match pattern {
                                None => {
                                    pattern = Some(lit);
                                }
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        lit,
                                        "multiple patterns spicifed, expected one.",
                                    ))
                                }
                            }
                        } else {
                            return Err(syn::Error::new_spanned(
                                name_value.lit,
                                "attribute pattern expects &str.",
                            ));
                        }
                    } else if name_value.path.is_ident("is_regex") {
                        if let syn::Lit::Bool(lit) = name_value.lit {
                            is_regex = Some(lit.value())
                        } else {
                            return Err(syn::Error::new_spanned(
                                name_value.lit,
                                "attribute is_regex expects bool.",
                            ));
                        }
                    } else if name_value.path.is_ident("is_command") {
                        if let syn::Lit::Bool(lit) = name_value.lit {
                            is_command = Some(lit.value())
                        } else {
                            return Err(syn::Error::new_spanned(
                                name_value.lit,
                                "attribute is_command expects bool.",
                            ));
                        }
                    }
                }
                arg => return Err(syn::Error::new_spanned(arg, "unknown attribute.")),
            }
        }

        if pattern.is_none() {
            return Err(syn::Error::new(
                Span::call_site(),
                "attribute pattern expected.",
            ));
        }

        Ok(Self {
            pattern: pattern.unwrap(),
            is_regex: match is_regex {
                Some(v) => v,
                None => false,
            },
            is_command: match is_command {
                Some(v) => v,
                None => false,
            },
        })
    }
}

pub(crate) struct Update {
    name: Ident,
    args: Args,
    ast: ItemFn,
    update_type: UpdateType,
}

impl Update {
    fn new(args: AttributeArgs, input: TokenStream, update_type: UpdateType) -> syn::Result<Self> {
        if args.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                format!(
                    r#"invalid handler definition, expected #[{}(pattern = "<some pattern>")]"#,
                    update_type.as_str().to_ascii_lowercase()
                ),
            ));
        }

        let ast = syn::parse::<ItemFn>(input)?;
        let name = ast.sig.ident.clone();

        let args = Args::new(args)?;

        Ok(Self {
            name,
            args,
            ast,
            update_type,
        })
    }
}

impl ToTokens for Update {
    fn to_tokens(&self, output: &mut TokenStream2) {
        let Self {
            name,
            args:
                Args {
                    pattern,
                    is_regex,
                    is_command,
                },
            ast,
            update_type,
        } = self;

        let handler_name = name.to_string();
        let stream = quote! {
            #[allow(non_camel_case_types)]

            pub struct #name;

            impl #name {
                fn register(self, __client: grammers_client::Client) {

                    #ast
                    let __handler = grammers_plugins::Handler::new(#pattern, #update_type)
                        .name(#handler_name)
                        .is_regex(#is_regex)
                        .is_command(#is_command);

                    grammers_plugins::Manager::register(__handler, __client);
                }
            }
        };

        output.extend(stream);
    }
}

pub(crate) fn register(
    args: TokenStream,
    input: TokenStream,
    update_type: UpdateType,
) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);

    match Update::new(args, input, update_type) {
        Ok(update) => update.into_token_stream().into(),
        Err(err) => err.to_compile_error().into(),
    }
}
