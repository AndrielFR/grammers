// Copyright 2020 - developers of the `grammers` project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
mod update;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn message(args: TokenStream, input: TokenStream) -> TokenStream {
    update::register(args, input, update::UpdateType::Message)
}

#[proc_macro_attribute]
pub fn callback_query(args: TokenStream, input: TokenStream) -> TokenStream {
    update::register(args, input, update::UpdateType::CallbackQuery)
}

#[proc_macro_attribute]
pub fn inline_query(args: TokenStream, input: TokenStream) -> TokenStream {
    update::register(args, input, update::UpdateType::InlineQuery)
}
