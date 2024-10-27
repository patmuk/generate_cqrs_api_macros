use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::File;
use syn::Variant;

use crate::utils::get_enum::get_enum_by_ident_keyword;

pub(crate) fn generate_effect_enum(domain_struct_name: &str, ast: &File) -> TokenStream {
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

    // .map(|punctuated| format_ident!("{}{}", domain_struct_name, punctuated.value()))
    // let processing_effect = format_ident!("{processing_effect_enum}");

    quote! {

        pub enum Effect {
            #(#variants),*
            // #variants(RustAutoOpaque<#domain_struct_name>),
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

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

        let result = generate_effect_enum("MyDomainModel", &ast);
        let expected = quote! {
                pub enum Effect {
                    MyDomainModelRenderTodoList(RustAutoOpaque<MyDomainModel>),
                    MyDomainModelDeleteTodoList
            }
        };

        // let result = AST.with(|ast| generate_effect_enum("", &ast));

        assert_eq!(expected.to_string(), result.to_string());
    }
}
