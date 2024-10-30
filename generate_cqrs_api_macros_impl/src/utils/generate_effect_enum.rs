use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::File;
use syn::Ident;
use syn::Variant;

use crate::utils::get_enum::get_enum_by_ident_keyword;

pub(crate) fn generate_effect_enum(domain_struct_name: &Ident, ast: &File) -> (Ident, TokenStream) {
    let processing_effect_enum = get_enum_by_ident_keyword(ast, "Effect");

    let variants = processing_effect_enum
        .variants
        .into_pairs()
        .map(|punctuated| {
            let variant = punctuated.value();
            Variant {
                ident: format_ident!(
                    "{domain_struct_name}{}",
                    punctuated.value().ident.to_string()
                ),
                ..variant.clone()
            }
        })
        .collect::<Vec<Variant>>();

    (
        processing_effect_enum.ident,
        quote! {
            pub enum Effect {
                #(#variants),*
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
                pub enum TodoListEffect {
                    RenderTodoList(RustAutoOpaque<MyDomainModel>),
                    DeleteTodoList,
                    }         
                    "#,
        )
        .expect("test oracle should be parsable");

        let result = generate_effect_enum(&format_ident!("MyDomainModel"), &ast);
        let expected = (
            format_ident!("TodoListEffect"),
            quote! {
                    pub enum Effect {
                        MyDomainModelRenderTodoList(RustAutoOpaque<MyDomainModel>),
                        MyDomainModelDeleteTodoList
                }
            },
        );

        // let result = AST.with(|ast| generate_effect_enum("", &ast));

        assert_eq!(expected.0.to_string(), result.0.to_string());
        assert_eq!(expected.1.to_string(), result.1.to_string());
    }
}
