use log::debug;
use quote::ToTokens;
use syn::{File, Ident, ItemEnum};

pub(crate) fn get_enum_type_by_ident_keyword(ast: &File, keyword: &str) -> Ident {
    debug!("----------- get enum type by ident keyword {keyword}:");
    let result = ast
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Enum(item_enum) if item_enum.ident.to_string().contains(keyword) => {
                Some(item_enum.ident.clone())
            }
            _ => None,
        })
        .collect::<Vec<Ident>>();
    debug!("got {} idents: {:?}", result.len(), result);
    match result.len() {
        0 => panic!("No enum found! Needs to include '{keyword}' in its name."),
        1 => result[0].to_owned(),
        _ => {
            panic!("More than one {keyword} enum found! Please combine all {keyword} cases in one Enum. Found: {result:#?}")
        }
    }
}
// alternative way to identify the error enum (by looking for derive(thiserror))
#[allow(dead_code)]
pub(crate) fn get_enum_ident_by_derive_keyword(ast: &File, keyword: &str) -> Ident {
    debug!("----------- get enum idents for keyword {keyword}:");
    let result = ast
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Enum(item_enum)
                if item_enum.attrs.iter().any(|attribute| {
                    attribute.path().is_ident("derive")
                        && attribute.to_token_stream().to_string().contains(keyword)
                }) =>
            {
                Some(item_enum.ident.clone())
            }
            _ => None,
        })
        .collect::<Vec<Ident>>();
    match result.len() {
        0 => panic!("No enum found! Needs to include '{keyword}' in its name."),
        1 => result[0].to_owned(),
        _ => {
            panic!("More than one {keyword} enum found! Please combine all {keyword} cases in one Enum. Found: {result:#?}")
        }
    }
}
pub(crate) fn get_enum_by_ident_keyword(ast: &File, keyword: &str) -> ItemEnum {
    debug!("----------- get enum variants by ident keyword {keyword}:");
    let result = ast
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Enum(item_enum) if item_enum.ident.to_string().contains(keyword) => {
                Some(item_enum.clone())
            }
            _ => None,
        })
        .collect::<Vec<ItemEnum>>();
    match result.len() {
        0 => panic!("No enum found! Needs to include '{keyword}' in its name."),
        1 => result[0].to_owned(),
        _ => {
            panic!("More than one {keyword} enum found! Please combine all {keyword} cases in one Enum. Found {} enums.", result.len())
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::{format_ident, quote};

    use crate::parsing::get_enum::{get_enum_by_ident_keyword, get_enum_type_by_ident_keyword};

    #[test]
    fn get_enum_test() {
        let ast = syn::parse_file(
            r#"
        #[derive(Debug)]
         pub enum MyGoodProcessingError {
             #[error("Error during processing: {0}")]
             Error(String)
         }
         "#,
        )
        .expect("test oracle should be parsable");

        let result = get_enum_type_by_ident_keyword(&ast, "Error");
        assert_eq!(format_ident!("MyGoodProcessingError"), result);
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

        let result = get_enum_type_by_ident_keyword(&ast, "Error");

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

        let result = get_enum_type_by_ident_keyword(&ast, "Error");

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

        assert_eq!(expected.to_string(), result.to_string());
    }

    #[test]
    fn get_whole_enum_test() {
        let input = r#"
        #[derive(Debug)]
        pub enum MyGoodEffect {
            EffectOne,
            Effect(RustAutoOpaque<MyDomain>),
            }
            "#;
        let ast = syn::parse_file(input).expect("test oracle should be parsable");

        let result = get_enum_by_ident_keyword(&ast, "Effect");

        assert_eq!(quote! {#ast}.to_string(), quote! {#result}.to_string());
    }
}
