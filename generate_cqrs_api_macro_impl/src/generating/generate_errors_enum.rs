use log::debug;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::generate_api_macro_impl::ModelNEffects;
use crate::generate_api_macro_impl::ModelNEffectsNErrors;
use crate::parsing::get_enum::get_enum_type_by_ident_keyword;

pub(crate) fn generate_errors_enum(
    models_n_effects: Vec<ModelNEffects>,
) -> (Vec<ModelNEffectsNErrors>, TokenStream) {
    // }
    // fn generate_error_enum(ast: &File) -> (Ident, TokenStream) {
    let models_n_effects_n_errors: Vec<ModelNEffectsNErrors> = models_n_effects
        .into_iter()
        .map(|model| {
            let error_ident = get_enum_type_by_ident_keyword(&model.ast, "Error");
            // let processing_error_enum = get_enum_type_by_ident_keyword(ast, "Error");
            debug!("----------- processing error enum(s): {:#?}\n", error_ident);

            ModelNEffectsNErrors {
                error_ident,
                ast: model.ast,
                base_path: model.base_path,
                domain_model_ident: model.domain_model_ident,
                effect_ident: model.effect_ident,
                effect_variants: model.effect_variants,
            }
        })
        .collect();

    // we are importing * from the base path, so this is imported already
    // otherwise, we need to know which Error belongs to which Model of which base path
    // let use_statement = generate_use_statement(base_path, &processing_error_enum.to_string());

    let processing_error = models_n_effects_n_errors
        .iter()
        .map(|model| model.error_ident.clone())
        .collect::<Vec<Ident>>();
    (
        models_n_effects_n_errors,
        quote! {
            // #use_statement
            #[derive(thiserror::Error, Debug)]
            pub enum ProcessingError {
                #[error("Error during processing: {0}")]
                #(#processing_error ( #processing_error ),)*
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
        generate_api_macro_impl::{BasePath, ModelNEffects},
        generating::generate_errors_enum::generate_errors_enum,
    };

    #[test]
    fn generate_error_enum_test_one_model() {
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

        let result = generate_errors_enum(vec![ModelNEffects {
            base_path: BasePath("".to_string()),
            ast,
            domain_model_ident: format_ident!("MyGoodDomain"),
            effect_ident: format_ident!("MyGoodDomainEffect"),
            effect_variants: vec![],
        }]);
        let expected_code = quote! {
            #[derive(thiserror::Error, Debug)]
            pub enum ProcessingError {
                #[error("Error during processing: {0}")]
                MyGoodProcessingError(MyGoodProcessingError),
                #[error("Processing was fine, but state could not be persisted: {0}")]
                NotPersisted(#[source] std::io::Error),
            }
        };
        assert_eq!(expected_code.to_string(), result.1.to_string());
    }

    #[test]
    fn generate_error_enum_test_two_models() {
        let ast_one = syn::parse_file(
            r#"
        #[derive(thiserror::Error, Debug)]
         pub enum MyGoodProcessingError {
            #[error("Error in MyGoodError during processing: {0}")]
            Error(String)
         }
         "#,
        )
        .expect("test oracle ast_one should be parsable");
        let ast_two = syn::parse_file(
            r#"
        #[derive(thiserror::Error, Debug)]
         pub enum MySecondProcessingError {
            #[error("Second Error during processing: {0}")]
            InnerError(String)
         }
         "#,
        )
        .expect("test oracle ast_two should be parsable");

        let result = generate_errors_enum(vec![
            ModelNEffects {
                ast: ast_one,
                domain_model_ident: format_ident!("MyGoodDomain"),
                effect_ident: format_ident!("MyGoodDomainEffect"),
                base_path: BasePath("".to_string()),
                effect_variants: vec![],
            },
            ModelNEffects {
                ast: ast_two,
                domain_model_ident: format_ident!("MySecondDomain"),
                effect_ident: format_ident!("MySecondDomainEffect"),
                base_path: BasePath("".to_string()),
                effect_variants: vec![],
            },
        ]);
        let expected_code = quote! {
            #[derive(thiserror::Error, Debug)]
            pub enum ProcessingError {
                #[error("Error during processing: {0}")]
                MyGoodProcessingError(MyGoodProcessingError),
                MySecondProcessingError(MySecondProcessingError),
                #[error("Processing was fine, but state could not be persisted: {0}")]
                NotPersisted(#[source] std::io::Error),
            }
        };
        assert_eq!(expected_code.to_string(), result.1.to_string());
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

        let result = generate_errors_enum(vec![ModelNEffects {
            base_path: BasePath("".to_string()),
            ast,
            domain_model_ident: format_ident!("MyGoodDomain"),
            effect_ident: format_ident!("MyGoodDomainEffect"),
            effect_variants: vec![],
        }]);

        let expected_code = quote! {
            #[derive(thiserror::Error, Debug)]
            pub enum ProcessingError {
                #[error("Error during processing: {0}")]
                MyGoodProcessingError(MyGoodProcessingError),
                #[error("Processing was fine, but state could not be persisted: {0}")]
                NotPersisted(#[source] std::io::Error),
            }
        };
        assert_eq!(expected_code.to_string(), result.1.to_string());
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

        let result = generate_errors_enum(vec![ModelNEffects {
            base_path: BasePath("".to_string()),
            ast,
            domain_model_ident: format_ident!("MyGoodDomain"),
            effect_ident: format_ident!("MyGoodDomainEffect"),
            effect_variants: vec![],
        }]);

        let expected_code = quote! {
            #[derive(thiserror::Error, Debug)]
            pub enum ProcessingError {
                #[error("Error during processing: {0}")]
                MyGoodProcessingError(MyGoodProcessingError),
                #[error("Processing was fine, but state could not be persisted: {0}")]
                NotPersisted(#[source] std::io::Error),
            }
        };
        assert_eq!(expected_code.to_string(), result.1.to_string());
    }
}