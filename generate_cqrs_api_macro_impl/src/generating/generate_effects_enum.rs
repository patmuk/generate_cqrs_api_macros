use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::Variant;

use crate::generate_api_macro_impl::ModelNEffects;
use crate::generate_api_macro_impl::ModelParsed;
use crate::parsing::get_enum::get_enum_by_ident_keyword;

pub(crate) fn generate_effects_enum(
    models_parsed: Vec<ModelParsed>,
) -> (Vec<ModelNEffects>, TokenStream) {
    let models_n_effects = models_parsed
        .iter()
        .map(|model_parsed| {
            let processing_effect_enum = get_enum_by_ident_keyword(&model_parsed.ast, "Effect");

            let variants = processing_effect_enum
                .variants
                .into_pairs()
                .map(|punctuated| punctuated.value().to_owned())
                .collect::<Vec<Variant>>();

            ModelNEffects {
                base_path: model_parsed.base_path.to_owned(),
                ast: model_parsed.ast.to_owned(),
                domain_model_ident: model_parsed.domain_model_ident.to_owned(),
                effect_ident: processing_effect_enum.ident,
                effect_variants: variants,
            }
        })
        .collect::<Vec<ModelNEffects>>();

    let prefixed_variants = models_n_effects
        .iter()
        .flat_map(|model_n_effects| {
            model_n_effects
                .effect_variants
                .iter()
                //.map(|variant| {
                .map(|variant| Variant {
                    ident: format_ident!(
                        "{}{}",
                        model_n_effects.domain_model_ident,
                        variant.ident.to_string()
                    ),
                    ..variant.clone()
                })
        })
        .collect::<Vec<Variant>>();
    (
        models_n_effects,
        quote! {
            pub enum Effect {
                #(#prefixed_variants),*
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use quote::{format_ident, quote};
    // use syn::{parse2, ItemEnum, Variant};

    use crate::{
        generate_api_macro_impl::{BasePath, ModelParsed},
        generating::generate_effects_enum::generate_effects_enum,
    };

    #[test]
    fn generate_effect_enum_test_one_model() {
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
        // let expected_effect_variants = parse2::<ItemEnum>(quote! {
        //     #[derive(Debug)]
        //     pub enum MyDomainModelEffect {
        //         RenderItemList(RustAutoOpaque<MyDomainModel>),
        //         DeleteItemList,
        //     }
        // })
        // .expect("Couldn't parse test oracle for expeceted effect variants!")
        // .variants
        // .into_pairs()
        // .map(|punctuated| punctuated.value().to_owned())
        // .collect::<Vec<Variant>>();

        let expected_effect_ident = format_ident!("MyDomainModelEffect");

        assert!(result.0.len() == 1);
        assert_eq!(expected_code.to_string(), result.1.to_string());
        assert_eq!(expected_effect_ident, result.0[0].effect_ident);
        // this assertion only works if syn's "extra-traits" feature is enabled.
        // as long as the expected_code is correct, this should be correct as well.
        // assert_eq!(expected_effect_variants, result.0[0].effect_variants);
    }

    #[test]
    fn generate_effect_enum_test_two_models() {
        let model_one = syn::parse_file(
            r#"
                #[derive(Debug)]
                pub enum MyDomainModelEffect {
                    RenderItemList(RustAutoOpaque<MyDomainModel>),
                    DeleteItemList,
                }
            "#,
        )
        .expect("test oracle should be parsable");
        let model_two = syn::parse_file(
            r#"
                #[derive(Debug)]
                pub enum MySecondModelEffect {
                    RenderObjectsist(RustAutoOpaque<MySecondModel>),
                    DuplicateObjectList,
                }
            "#,
        )
        .expect("test oracle should be parsable");

        let result = generate_effects_enum(vec![
            ModelParsed {
                domain_model_ident: format_ident!("MyDomainModel"),
                ast: model_one.clone(),
                base_path: BasePath("".to_string()),
            },
            ModelParsed {
                domain_model_ident: format_ident!("MySecondModel"),
                ast: model_two.clone(),
                base_path: BasePath("".to_string()),
            },
        ]);
        let expected_code = quote! {
            pub enum Effect {
                MyDomainModelRenderItemList(RustAutoOpaque<MyDomainModel>),
                MyDomainModelDeleteItemList,
                MySecondModelRenderObjectsist(RustAutoOpaque<MySecondModel>),
                MySecondModelDuplicateObjectList
            }
        };
        // let expected_effect_variants_one = parse2::<ItemEnum>(quote! {
        //     #[derive(Debug)]
        //     pub enum MyDomainModelEffect {
        //         RenderItemList(RustAutoOpaque<MyDomainModel>),
        //         DeleteItemList,
        //     }
        // })
        // .expect("Couldn't parse test oracle for expeceted effect variants!")
        // .variants
        // .into_pairs()
        // .map(|punctuated| punctuated.value().to_owned())
        // .collect::<Vec<Variant>>();
        // let expected_effect_variants_two = parse2::<ItemEnum>(quote! {
        //     #[derive(Debug)]
        //     pub enum MySecondModelEffect {
        //         RenderObjectsist(RustAutoOpaque<MySecondModel>),
        //         DuplicateObjectList,
        //     }
        // })
        // .expect("Couldn't parse test oracle for expeceted effect variants!")
        // .variants
        // .into_pairs()
        // .map(|punctuated| punctuated.value().to_owned())
        // .collect::<Vec<Variant>>();

        let expected_effect_ident_one = format_ident!("MyDomainModelEffect");
        let expected_effect_ident_two = format_ident!("MySecondModelEffect");

        assert!(result.0.len() == 2);
        assert_eq!(expected_code.to_string(), result.1.to_string());
        assert_eq!(expected_effect_ident_one, result.0[0].effect_ident);
        assert_eq!(expected_effect_ident_two, result.0[1].effect_ident);
        // this assertion only works if syn's "extra-traits" feature is enabled.
        // as long as the expected_code is correct, this should be correct as well.
        // assert_eq!(expected_effect_variants_one, result.0[0].effect_variants);
        // assert_eq!(expected_effect_variants_two, result.0[1].effect_variants);
    }
}
