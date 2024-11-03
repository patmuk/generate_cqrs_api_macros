use proc_macro2::Span;
use quote::format_ident;
use syn::{File, Ident, Path, PathArguments, Result, Type};

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
                    .map(|tipe| get_ident(&tipe))
                    .collect::<Vec<Result<Ident>>>(),
            ),
        )),
    }
}

/// extracts the path from a type
pub(crate) fn get_path(tipe: &Type) -> Result<Path> {
    match tipe {
        syn::Type::Path(type_path) => Ok(type_path.path.to_owned()),
        _ => Err(syn::Error::new(Span::call_site(), "Not a struct type.")),
    }
}
/// extracts the ident (name) of a type, if existing
pub(crate) fn get_ident(tipe: &Type) -> Result<Ident> {
    get_path(tipe)?
        .get_ident()
        .ok_or_else(|| syn::Error::new(Span::call_site(), "item has no ident"))
        .cloned()
}

/// converts the type's type ident into a name
/// e.g. Foo -> foo
pub(crate) fn get_type_name(tipe: &Type) -> Result<Ident> {
    Ok(format_ident!(
        "{}",
        stringcase::snake_case(&get_inner_type_name(tipe)?)
    ))
}
fn get_inner_type_name(tipe: &Type) -> Result<String> {
    match tipe {
        Type::Array(type_array) => get_inner_type_name(&*type_array.elem),
        Type::Slice(type_slice) => get_inner_type_name(&*type_slice.elem),
        Type::Group(type_group) => get_inner_type_name(&*type_group.elem),
        Type::Paren(type_paren) => get_inner_type_name(&*type_paren.elem),
        Type::Ptr(type_ptr) => get_inner_type_name(&*type_ptr.elem),
        Type::Reference(type_reference) => Ok(get_inner_type_name(&*type_reference.elem)? + "_"),
        Type::Tuple(type_tuple) => Ok({
            let mut elements = type_tuple.elems.iter();
            let first = get_inner_type_name(elements.next().unwrap())?;
            elements.try_fold(first, |acc, e| match get_inner_type_name(e) {
                Ok(s) => Ok(acc + "_" + &s), // <-- added underscore before concatenating
                Err(err) => Err(err),
            })?
        }),
        Type::Path(type_path) => {
            let last_segment = type_path
                .path
                .segments
                .last()
                .expect("should be a type, but there was no last path segment!");
            match &last_segment.arguments {
                PathArguments::None => Ok(last_segment.ident.to_string()),
                PathArguments::AngleBracketed(angled_args) => {
                    angled_args
                        .args
                        .iter()
                        .try_fold(last_segment.ident.to_string(), |acc, e| match e {
                            syn::GenericArgument::Type(tipe) => {
                                get_inner_type_name(tipe).map(|inner_type| acc + "_" + &inner_type)
                            }
                            _ => Err(syn::Error::new(
                                Span::call_site(),
                                "Not a supported type in this angle_bracketed_agrument.",
                            )),
                        })
                }
                PathArguments::Parenthesized(_) => Err(syn::Error::new(
                    Span::call_site(),
                    "Parenthesized types are not supported.",
                )),
            }
        }
        _ => Err(syn::Error::new(Span::call_site(), "Not a supported type.")),
    }
}

#[cfg(test)]
mod tests {
    use syn::{parse_str, ItemType, Type};

    use crate::utils::get_domain_model_struct::get_domain_model_struct_ident;

    use super::get_type_name;

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

    #[test]
    fn get_type_name_test_simple_type() {
        let input = parse_str::<Type>("Foo").expect("test oracle should be parsable");
        let name = get_type_name(&input);
        assert_eq!("foo", &name.unwrap().to_string());
    }
    #[test]
    fn get_type_name_test_vec_type() {
        let input = parse_str::<Type>("Vec<Foo>").expect("test oracle should be parsable");
        let name = get_type_name(&input);
        assert_eq!("vec_foo", &name.unwrap().to_string());
    }
    #[test]
    fn get_type_name_test_vec_type_with_type() {
        let input = parse_str::<Type>("Vec<Foo<Bar>>").expect("test oracle should be parsable");
        let name = get_type_name(&input);
        assert_eq!("vec_foo_bar", &name.unwrap().to_string());
    }
    #[test]
    fn get_type_name_test_two_tuple_type() {
        let input = parse_str::<Type>("(Foo, Bar)").expect("test oracle should be parsable");
        let name = get_type_name(&input);
        assert_eq!("foo_bar", &name.unwrap().to_string());
    }
    #[test]
    fn get_type_name_test_slice_type() {
        let input = parse_str::<Type>("[Foo]").expect("test oracle should be parsable");
        let name = get_type_name(&input);
        assert_eq!("foo", &name.unwrap().to_string());
    }
    #[test]
    fn get_type_name_test_two_tuple_vec_type() {
        let input = parse_str::<Type>("(Vec<Foo>, Bar)").expect("test oracle should be parsable");
        let name = get_type_name(&input);
        assert_eq!("vec_foo_bar", &name.unwrap().to_string());
    }
    #[test]
    fn get_type_name_test_two_tuple_two_vec_type() {
        let input =
            parse_str::<Type>("(Vec<Foo>, Vec<Bar>)").expect("test oracle should be parsable");
        let name = get_type_name(&input);
        assert_eq!("vec_foo_vec_bar", &name.unwrap().to_string());
    }
    #[test]
    fn get_type_name_test_two_tuple_vec_type_with_type() {
        let input =
            parse_str::<Type>("(Vec<Foo<Bar>>, Vec<Bar>)").expect("test oracle should be parsable");
        let name = get_type_name(&input);
        assert_eq!("vec_foo_bar_vec_bar", &name.unwrap().to_string());
    }
}
