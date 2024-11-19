use log::debug;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::File;
use syn::Ident;

use crate::generate_api_macro_impl::BasePath;
use crate::utils::generate_use_statement::generate_use_statement;
use crate::utils::get_enum::get_enum_type_by_ident_keyword;

pub(crate) fn generate_error_enum(base_path: &BasePath, ast: &File) -> (Ident, TokenStream) {
    let processing_error_enum = get_enum_type_by_ident_keyword(ast, "Error");

    debug!(
        "----------- processing error enum(s): {:#?}\n",
        processing_error_enum
    );

    let use_statement = generate_use_statement(base_path, &processing_error_enum.to_string());
    let processing_error = format_ident!("{processing_error_enum}");

    (
        processing_error.clone(),
        quote! {
            #use_statement

            #[derive(thiserror::Error, Debug)]
            pub enum ProcessingError {
                #[error("Error during processing: {0}")]
                #processing_error(#processing_error),
                #[error("Processing was fine, but state could not be persisted: {0}")]
                NotPersisted(#[source] std::io::Error),
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use quote::{format_ident, quote};

    use crate::{
        generate_api_macro_impl::BasePath, utils::generate_error_enum::generate_error_enum,
    };

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

        let result = generate_error_enum(&BasePath("crate::Model".to_string()), &ast);
        let expected = (
            format_ident!("MyGoodProcessingError"),
            quote! {
                use crate::Model::MyGoodProcessingError;

                #[derive(thiserror::Error, Debug)]
                pub enum ProcessingError {
                    #[error("Error during processing: {0}")]
                    MyGoodProcessingError(MyGoodProcessingError),
                    #[error("Processing was fine, but state could not be persisted: {0}")]
                    NotPersisted(#[source] std::io::Error),
                }
            },
        );

        assert_eq!(expected.0.to_string(), result.0.to_string());
        assert_eq!(expected.1.to_string(), result.1.to_string());
    }
    #[test]
    #[should_panic(
        expected = r#"More than one Error enum found! Please combine all Error cases in one Enum. Found: [
    Ident(
        ProcessingError,
    ),
    Ident(
        SecondProcessingError,
    ),
]"#
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

        let result = generate_error_enum(&BasePath("crate::Model".to_string()), &ast);

        let expected = (
            format_ident!("MyGoodProcessingError"),
            quote! {
                use::MyGoodProcessingError;

                #[derive(thiserror::Error, Debug)]
                pub enum ProcessingError {
                    #[error("Error during processing: {0}")]
                    MyGoodProcessingError(MyGoodProcessingError),
                    #[error("Processing was fine, but state could not be persisted: {0}")]
                    NotPersisted(#[source] std::io::Error),
                }
            },
        );
        assert_eq!(expected.0.to_string(), result.0.to_string());
        assert_eq!(expected.1.to_string(), result.1.to_string());
    }
    #[test]
    #[should_panic(expected = "No enum found! Needs to include 'Error' in its name.")]
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

        let result = generate_error_enum(&BasePath("crate::Model".to_string()), &ast);

        let expected = (
            format_ident!("MyGoodProcessingError"),
            quote! {
                use::MyGoodProcessingError;

                #[derive(thiserror::Error, Debug)]
                pub enum ProcessingError {
                    #[error("Error during processing: {0}")]
                    MyGoodProcessingError(MyGoodProcessingError),
                    #[error("Processing was fine, but state could not be persisted: {0}")]
                    NotPersisted(#[source] std::io::Error),
                }
            },
        );

        assert_eq!(expected.0.to_string(), result.0.to_string());
        assert_eq!(expected.1.to_string(), result.1.to_string());
    }
}
