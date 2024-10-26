use log::debug;
use proc_macro2::TokenStream;
use quote::quote;
use quote::{format_ident, ToTokens};
use syn::{parse_str, File};

use crate::utils::generate_use_statement::generate_use_statement;

pub(crate) fn generate_error_enum(base_path: &str, ast: &File) -> TokenStream {
    let processing_error_enum_idents = get_processing_error_enum_idents(ast);
    // TODO support multiplr inputs

    debug!(
        "----------- processing error enum(s): {:#?}\n",
        processing_error_enum_idents
    );

    // TODO refactor for multiple or single
    let processing_error_string = processing_error_enum_idents
        .first()
        .expect("no processing error enum found");

    let use_statement = generate_use_statement(&base_path, &processing_error_string);
    let processing_error = format_ident!("{processing_error_string}");

    quote! {
        #use_statement

        #[derive(thiserror::Error, Debug)]
        pub enum ProcessingError {
            #[error("Error during processing: {0}")]
            #processing_error(#processing_error),
            #[error("Processing was fine, but state could not be persisted: {0}")]
            NotPersisted(#[source] std::io::Error),
        }
    }
}

/// searches the error enum(s) by derive thiserror only!
// / TODO search for the word "Error" in the name?
/// TODO search for one error enum, panic if more are present?
fn get_processing_error_enum_idents(ast: &File) -> Vec<String> {
    println!("----------- get processing error enum idents:");
    ast.items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Enum(item_enum)
                if item_enum.attrs.iter().any(|attribute| {
                    attribute.path().is_ident("derive")
                        && attribute
                            .to_token_stream()
                            .to_string()
                            .contains("thiserror")
                }) =>
            {
                Some(item_enum.ident.to_string())
            }
            _ => None,
        })
        .collect::<Vec<String>>()
}
