extern crate proc_macro;

use generate_cqrs_api_macro_impl::generate_api_macro_impl;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn generate_api(file_paths: TokenStream, item: TokenStream) -> proc_macro::TokenStream {
    TokenStream::from(
        generate_api_macro_impl::generate_api_impl(
            proc_macro2::TokenStream::from(item),
            proc_macro2::TokenStream::from(file_paths),
        )
        .unwrap_or_else(|e| e.to_compile_error()),
    )
}
