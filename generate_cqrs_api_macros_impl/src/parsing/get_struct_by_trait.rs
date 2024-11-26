use syn::{parse_file, File, Ident, ItemImpl};

use super::type_2_ident::get_ident;

pub(crate) fn get_structs_by_traits(ast: &File, trait_idents: &[&str]) -> Vec<Ident> {
    ast.items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item_impl) if item_impl.trait_.is_some() => Some(item_impl),
            _ => None,
        })
        .filter_map(|item_impl| {
            item_impl.trait_.as_ref().and_then(|trait_i| {
                trait_i.1.segments.last().and_then(|segment| {
                    if trait_idents.contains(&segment.ident.to_string().as_str()) {
                        get_ident(&item_impl.self_ty).ok()
                    } else {
                        None
                    }
                })
            })
        })
        .collect::<Vec<Ident>>()

    // .collect::<Vec<String>>()
}

#[cfg(test)]
#[test]
fn good_test_two_structs() {
    let ast = parse_file(
        r#"
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
    "#,
    )
    .unwrap();

    let structs = get_structs_by_traits(&ast, &["ModelTrait", "DifferentTrait"]);

    assert_eq!(structs, ["Model", "SomethingDifferent"]);
}
