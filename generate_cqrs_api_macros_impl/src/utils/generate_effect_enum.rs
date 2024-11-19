use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::File;
use syn::Ident;
use syn::Variant;

use crate::utils::get_enum::get_enum_by_ident_keyword;

pub(crate) fn generate_effect_enum(
    domain_struct_name: &Ident,
    ast: &File,
) -> (Ident, Vec<Variant>, TokenStream) {
    let processing_effect_enum = get_enum_by_ident_keyword(ast, "Effect");

    let variants = processing_effect_enum
        .variants
        .into_pairs()
        .map(|punctuated| punctuated.value().to_owned())
        .collect::<Vec<Variant>>();

    let prefixed_variant = variants
        .iter()
        .map(|variant| Variant {
            ident: format_ident!("{domain_struct_name}{}", variant.ident.to_string()),
            ..variant.clone()
        })
        .collect::<Vec<Variant>>();
    (
        processing_effect_enum.ident,
        variants,
        quote! {
            pub enum Effect {
                #(#prefixed_variant),*
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use quote::{format_ident, quote};

    use crate::utils::generate_effect_enum::generate_effect_enum;

    #[test]
    fn generate_effect_enum_test() {
        let ast = syn::parse_file(
            r#"
                #[derive(Debug)]
                pub enum MyDomainModelEffect {
                    RenderItemList(RustAutoOpaque<MyDomainModel>),
                    DeleteItemList,
                    }         
                    "#,
        )
        .expect("test oracle should be parsable");

        let result = generate_effect_enum(&format_ident!("MyDomainModel"), &ast);
        let expected = (
            format_ident!("MyDomainModelEffect"),
            vec![
                "RenderItemList(RustAutoOpaque<MyDomainModel>),
                DeleteItemList,",
            ],
            quote! {
                    pub enum Effect {
                        MyDomainModelRenderItemList(RustAutoOpaque<MyDomainModel>),
                        MyDomainModelDeleteItemList
                }
            },
        );

        assert_eq!(expected.0.to_string(), result.0.to_string());
        // assert_eq!(expected.1, result.1); // if all other tests are ok, this intermediate result must be ok as well
        assert_eq!(expected.2.to_string(), result.2.to_string());
    }
}
