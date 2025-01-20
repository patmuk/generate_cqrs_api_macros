use std::collections::HashMap;
use syn::{File, Ident};

use super::extract_type::get_type_as_capital_ident;

pub(crate) fn get_structs_by_traits(ast: &File, trait_idents: &[&str]) -> HashMap<String, Ident> {
    let found_structs = ast
        .items
        .iter()
        // getting all impls of traits
        .filter_map(|item| match item {
            syn::Item::Impl(item_impl) if item_impl.trait_.is_some() => Some(item_impl),
            _ => None,
        })
        // filtering for the relevant traits and creating a HashMap "trait: struct"
        .fold(HashMap::new(), |mut acc, item_impl| {
            if let Some(trait_impl) = &item_impl.trait_ {
                // like syn::path::get_ident(Path) we assume that the first element is the trait.
                // however, unlike syn::path::get_ident(Path) we don't mind if this element has angeled brackets
                if let Some(first_segment) = trait_impl.1.segments.first() {
                    let trait_ident = &first_segment.ident;
                    if trait_idents.contains(&trait_ident.to_string().as_str()) {
                        acc.insert(
                            trait_ident.to_string(),
                            get_type_as_capital_ident(&item_impl.self_ty)
                                .expect("Couldn't get type"),
                        );
                    }
                }
            }
            acc
        });

    if found_structs.len() != trait_idents.len() {
        // sort the resulting hashmap for consistant error output
        let found_structs_string = if cfg!(test) {
            let mut found_structs_sorted = "".to_string();
            let mut sorted_keys = found_structs.keys().cloned().collect::<Vec<String>>();
            sorted_keys.sort();
            for key in sorted_keys {
                if let Some(entry) = found_structs.get(&key) {
                    found_structs_sorted += &format!("{}: {:?}, ", key, entry);
                }
            }
            found_structs_sorted
        } else {
            format!("{:?}", found_structs)
        };
        panic!("Not all traits are implemented.\n Searched impl of {:?}\n but found only these implementing structs {:?}", trait_idents, found_structs_string)
    } else {
        found_structs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_file;

    const AST_STR: &str = r#"
trait ModelTrait {
    pub fn foo() -> String;
    }
    
    struct Model{}
    struct OtherModel{}
    impl ModelTrait for Model {
        fn foo() -> String {
            "foo".to_string()
            }
        }
        trait DifferentTrait{}
        struct SomethingDifferent{}
        impl DifferentTrait for SomethingDifferent{}
        trait NotRequestedTrait{}
        struct SomethingCompletelyDifferent{}
        impl NotRequestedTrait for SomethingCompletelyDifferent{}
        "#;

    #[test]
    fn good_test_two_structs() {
        let ast = parse_file(AST_STR).unwrap();

        let structs = get_structs_by_traits(&ast, &["ModelTrait", "DifferentTrait"]);
        let structs_string: HashMap<String, String> = structs
            .into_iter()
            .map(|(key, value)| (key, value.to_string()))
            .collect();
        assert_eq!(
            structs_string,
            HashMap::from([
                ("ModelTrait".to_string(), "Model".to_string()),
                (
                    "DifferentTrait".to_string(),
                    "SomethingDifferent".to_string()
                )
            ])
        );
    }
    #[test]
    #[should_panic = r#"Not all traits are implemented.
 Searched impl of ["ModelTrait", "DifferentTrait", "NotImpl"]
 but found only these implementing structs "DifferentTrait: Ident(SomethingDifferent), ModelTrait: Ident(Model), "#]
    fn missing_trait_impl() {
        let ast_input = AST_STR.to_string() + "\n trait NotImpl{}";
        let ast = parse_file(&ast_input).unwrap();

        get_structs_by_traits(&ast, &["ModelTrait", "DifferentTrait", "NotImpl"]);
    }
}
