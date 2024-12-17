use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::Variant;

use crate::generate_api_macro_impl::ModelNEffects;
use crate::generate_api_macro_impl::ModelParsed;
use crate::parsing::get_enum::get_enum_by_ident_keyword;

pub(crate) fn generate_effects_enum(
    models_parsed: Vec<ModelParsed>,
) -> (ModelNEffects, TokenStream) {
    // TODO aply to all models
    let model_parsed = &models_parsed[0];

    let processing_effect_enum = get_enum_by_ident_keyword(&model_parsed.ast, "Effect");

    let variants = processing_effect_enum
        .variants
        .into_pairs()
        .map(|punctuated| punctuated.value().to_owned())
        .collect::<Vec<Variant>>();

    let prefixed_variant = variants
        .iter()
        .map(|variant| Variant {
            ident: format_ident!(
                "{}{}",
                model_parsed.domain_model_ident,
                variant.ident.to_string()
            ),
            ..variant.clone()
        })
        .collect::<Vec<Variant>>();
    (
        ModelNEffects {
            base_path: model_parsed.base_path.to_owned(),
            ast: model_parsed.ast.to_owned(),
            domain_model_ident: model_parsed.domain_model_ident.to_owned(),
            effect_ident: processing_effect_enum.ident,
            effect_variants: variants,
        },
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
    use syn::{parse2, ItemEnum, Variant};

    use crate::{
        generate_api_macro_impl::{BasePath, ModelParsed},
        generating::generate_effects_enum::generate_effects_enum,
    };

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

        let result = generate_effects_enum(vec![ModelParsed {
            domain_model_ident: format_ident!("MyDomainModel"),
            ast: ast.clone(),
            base_path: BasePath("".to_string()),
        }]);
        let expected_code = quote! {
                pub enum Effect {
                    MyDomainModelRenderItemList(RustAutoOpaque<MyDomainModel>),
                    MyDomainModelDeleteItemList
            }
        };
        let expected_effect_variants = parse2::<ItemEnum>(quote! {
            #[derive(Debug)]
            pub enum MyDomainModelEffect {
                RenderItemList(RustAutoOpaque<MyDomainModel>),
                DeleteItemList,
            }
        })
        .expect("Couldn't parse test oracle for expeceted effect variants!")
        .variants
        .into_pairs()
        .map(|punctuated| punctuated.value().to_owned())
        .collect::<Vec<Variant>>();

        let expected_effect_ident = format_ident!("MyDomainModelEffect");

        assert_eq!(expected_code.to_string(), result.1.to_string());
        assert_eq!(expected_effect_ident, result.0.effect_ident);
        assert_eq!(expected_effect_variants, result.0.effect_variants);
    }
}
