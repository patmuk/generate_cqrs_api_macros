use log::debug;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::File;

use crate::utils::generate_use_statement::generate_use_statement;
use crate::utils::get_enum::get_enum_type_by_ident_keyword;

pub(crate) fn generate_error_enum(base_path: &str, ast: &File) -> TokenStream {
    let processing_error_enum = get_enum_type_by_ident_keyword(ast, "Error");

    debug!(
        "----------- processing error enum(s): {:#?}\n",
        processing_error_enum
    );

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

        let result = generate_error_enum("crate::Model", &ast);
        let expected = quote! {
            use crate::Model::MyGoodProcessingError;

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
        expected = "More than one Error enum found! Please combine all Error cases in one Enum. Found: [\n    \"ProcessingError\",\n    \"SecondProcessingError\",\n]"
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

        let result = generate_error_enum("crate::Model", &ast);

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

        let result = generate_error_enum("crate::Model", &ast);

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
