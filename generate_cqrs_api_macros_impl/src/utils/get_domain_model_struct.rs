use proc_macro2::Span;
use syn::{File, Ident, Result, Type};

use super::type_2_ident::get_ident;

pub(crate) fn get_domain_model_struct_ident(ast: &File) -> Result<Ident> {
    let domain_model_type = ast
        .items
        .iter()
        .filter_map(|item| match item {
            // syn::Item::Impl(item_impl)
            syn::Item::Impl(item_impl)
                if item_impl.trait_.is_some()
                    && item_impl
                        .trait_
                        .clone()
                        .expect("Should have gotten a trait")
                        .1
                        .segments
                        .iter()
                        .any(|segment| segment.ident == "CqrsModel") =>
            {
                Some(item_impl.self_ty.clone())
            }
            _ => None,
        })
        .collect::<Vec<Box<Type>>>();
    match domain_model_type.len() {
        0 => Err(syn::Error::new(
            Span::call_site(),
            "No domain model struct found. Mark it with the trait CqrsModel.",
        )),
        1 => get_ident(&domain_model_type[0]),
        _ => Err(syn::Error::new(
            Span::call_site(),
            format!(
                "Only mark one struct as the domain model! Found {:#?}",
                domain_model_type
                    .iter()
                    .map(|tipe| get_ident(tipe))
                    .collect::<Vec<Result<Ident>>>(),
            ),
        )),
    }
}

#[cfg(test)]
mod tests {

    use crate::utils::get_domain_model_struct::get_domain_model_struct_ident;

    const DEFAULT_CODE: &str = r#"
            use::MyGoodProcessingError;

            #[derive(thiserror::Error, Debug)]
            pub enum ProcessingError {
                #[error("Error during processing: {0}")]
                MyGoodProcessingError(MyGoodProcessingError),
                #[error("Processing was fine, but state could not be persisted: {0}")]
                NotPersisted(#[source] std::io::Error),
            }

            trait CqrsModel {}
    "#;
    #[test]
    fn get_one_domain_model_test() {
        let input = r#"
            pub struct MyDomainModel{
                name: String,
                value: u32,
            }
            impl CqrsModel for MyDomainModel {}
        "#
        .to_string()
            + DEFAULT_CODE;
        let ast = syn::parse_file(&input).expect("test oracle should be parsable");
        let result = get_domain_model_struct_ident(&ast).unwrap();
        assert_eq!("MyDomainModel", result.to_string());
    }
    #[test]
    fn get_no_domain_model_test() {
        let input = r#"
            pub struct MyDomainModel{
                name: String,
                value: u32,
            }
        "#
        .to_string()
            + DEFAULT_CODE;
        let ast = syn::parse_file(&input).expect("test oracle should be parsable");
        let result = get_domain_model_struct_ident(&ast);
        // assert_eq!("aha", result.to_string());
        assert_eq!(
            "No domain model struct found. Mark it with the trait CqrsModel.",
            result.unwrap_err().to_string()
        );
    }

    #[test]
    fn get_two_domain_model_test() {
        let input = r#"
            pub struct MyDomainModel{
                name: String,
                value: u32,
            }
            impl CqrsModel for MyDomainModel {}
            
            pub struct SecondMyDomainModel{
                name: String,
                value: u32,
            }
            impl CqrsModel for SecondMyDomainModel {}

        "#
        .to_string()
            + DEFAULT_CODE;
        let ast = syn::parse_file(&input).expect("test oracle should be parsable");
        let result = get_domain_model_struct_ident(&ast);
        assert_eq!(
            r#"Only mark one struct as the domain model! Found [
    Ok(
        Ident(
            MyDomainModel,
        ),
    ),
    Ok(
        Ident(
            SecondMyDomainModel,
        ),
    ),
]"#,
            result.unwrap_err().to_string()
        );
    }
}
