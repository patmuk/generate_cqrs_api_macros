use log::debug;
use proc_macro2::TokenStream;
use quote::quote;
use quote::{format_ident, ToTokens};
use syn::File;

use crate::utils::generate_use_statement::generate_use_statement;

pub(crate) fn generate_error_enum(base_path: &str, ast: &File) -> TokenStream {
    let processing_error_enum = get_processing_error_enum_idents(ast);
    // TODO support multiplr inputs

    debug!(
        "----------- processing error enum(s): {:#?}\n",
        processing_error_enum
    );

    // TODO refactor for multiple or single

    let use_statement = generate_use_statement(base_path, &processing_error_enum);
    let processing_error = format_ident!("{processing_error_enum}");

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
fn get_processing_error_enum_idents(ast: &File) -> String {
    debug!("----------- get processing error enum idents:");
    let result = ast
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Enum(item_enum) if item_enum.ident.to_string().contains("Error") => {
                Some(item_enum.ident.to_string())
            }
            _ => None,
        })
        .collect::<Vec<String>>();
    debug!("got {} idents: {:?}", result.len(), result);
    match result.len() {
        0 => panic!("No error enum found! Needs to include 'Error' in its name."),
        1 => result[0].to_owned(),
        _ => {
            panic!("More than one error enum found! Please combine all error cases in one Enum. Found: {result:#?}")
        }
    }
}
// alternative way to identify the error enum (by looking for derive(thiserror))
#[allow(dead_code)]
fn get_processing_error_enum_idents_by_derive_attribute(ast: &File) -> String {
    debug!("----------- get processing error enum idents:");
    let result = ast
        .items
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
        .collect::<Vec<String>>();
    match result.len() {
        0 => panic!("No error enum found! Needs to derive from `thiserror`"),
        1 => result[0].to_owned(),
        _ => {
            panic!("More than one error enum found! Please combine all error cases in one Enum. Found: {result:#?}")
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use crate::utils::generate_error_enum::generate_error_enum;

    #[test]
    fn generate_error_enum_test() {
        let ast = syn::parse_file(
            r#"
        #[derive(thiserror::Error, Debug)]
         pub enum MyGoodProcessingError {
             #[error("Error during processing: {0}")]
             Error(String)
         }
         "#,
        )
        .expect("test oracle should be parsable");

        let result = generate_error_enum("", &ast);
        let expected = quote! {
            use::MyGoodProcessingError;

            #[derive(thiserror::Error, Debug)]
            pub enum ProcessingError {
                #[error("Error during processing: {0}")]
                MyGoodProcessingError(MyGoodProcessingError),
                #[error("Processing was fine, but state could not be persisted: {0}")]
                NotPersisted(#[source] std::io::Error),
            }
        };

        // let result = AST.with(|ast| generate_error_enum("", &ast));

        assert_eq!(expected.to_string(), result.to_string());
    }
    #[test]
    #[should_panic(
        expected = "More than one error enum found! Please combine all error cases in one Enum. Found: [\n    \"ProcessingError\",\n    \"SecondProcessingError\",\n]"
    )]
    fn fail_more_then_one_error_enum_test() {
        let ast = syn::parse_file(
            r#"
        #[derive(thiserror::Error, Debug)]
         pub enum ProcessingError {
             #[error("Error during processing: {0}")]
             Error(String)
         }
        #[derive(thiserror::Error, Debug)]
         pub enum SecondProcessingError {
             #[error("Second Error during processing: {0}")]
             AnotherError(String)
         }
         "#,
        )
        .expect("test oracle should be parsable");

        let result = generate_error_enum("", &ast);

        let expected = quote! {
            use::MyGoodProcessingError;

            #[derive(thiserror::Error, Debug)]
            pub enum ProcessingError {
                #[error("Error during processing: {0}")]
                MyGoodProcessingError(MyGoodProcessingError),
                #[error("Processing was fine, but state could not be persisted: {0}")]
                NotPersisted(#[source] std::io::Error),
            }
        };

        // let result = AST.with(|ast| generate_error_enum("", &ast));

        assert_eq!(expected.to_string(), result.to_string());
    }
    #[test]
    #[should_panic(expected = "No error enum found! Needs to include 'Error' in its name.")]
    fn fail_no_error_enum_test() {
        let ast = syn::parse_file(
            r#"
        #[derive(Debug)]
         pub enum MyGoodProcessingFailure {
             #[error("Error during processing: {0}")]
             Error(String)
         }
         "#,
        )
        .expect("test oracle should be parsable");

        let result = generate_error_enum("", &ast);

        let expected = quote! {
            use::MyGoodProcessingError;

            #[derive(thiserror::Error, Debug)]
            pub enum ProcessingError {
                #[error("Error during processing: {0}")]
                MyGoodProcessingError(MyGoodProcessingError),
                #[error("Processing was fine, but state could not be persisted: {0}")]
                NotPersisted(#[source] std::io::Error),
            }
        };

        // let result = AST.with(|ast| generate_error_enum("", &ast));

        assert_eq!(expected.to_string(), result.to_string());
    }
}
