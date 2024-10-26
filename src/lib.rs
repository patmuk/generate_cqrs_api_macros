extern crate proc_macro;

use generate_cqrs_api_macros_impl::generate_api_macro_impl;
use proc_macro::TokenStream;

#[proc_macro]
pub fn generate_api(file_pathes: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let file_pathes = proc_macro2::TokenStream::from(file_pathes);
    TokenStream::from(
        generate_api_macro_impl::generate_api_impl(file_pathes)
            .unwrap_or_else(|e| e.to_compile_error()),
    )
}
