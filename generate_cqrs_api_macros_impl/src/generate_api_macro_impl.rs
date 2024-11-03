use log::debug;

use crate::utils::generate_cqrs_impl::generate_cqrs_impl;
use crate::utils::generate_effect_enum::generate_effect_enum;
use crate::utils::generate_error_enum::generate_error_enum;
use crate::utils::get_domain_model_struct::get_domain_model_struct_ident;
use crate::utils::read_rust_files::{read_rust_file_content, tokens_2_file_locations};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

pub(crate) struct BasePath(pub(crate) String);
pub(crate) struct SourceCode(pub(crate) String);

pub fn generate_api_impl(file_pathes: TokenStream) -> Result<TokenStream> {
    simple_logger::init_with_level(log::Level::Debug).expect("faild to init logger");

    log::info!("-------- Generating API --------");

    let file_locations = tokens_2_file_locations(file_pathes)?;
    let (base_path, file_content) = read_rust_file_content(&file_locations[0])?;

    let generated_code = generate_code(base_path, file_content)?;
    Ok(generated_code)
}

fn generate_code(base_path: BasePath, file_content: SourceCode) -> Result<TokenStream> {
    let ast = syn::parse_file(&file_content.0)?;
    let domain_model_struct_ident = get_domain_model_struct_ident(&ast)
        .expect("Couldn't extract the domain model's name. One Struct needs to derive CqrsModel.");
    debug!("domain model name: {:#?}", domain_model_struct_ident);
    let (effect_ident, effect_variants, generated_effect_enum) =
        generate_effect_enum(&domain_model_struct_ident, &ast);
    let (error_ident, generated_error_enum) = generate_error_enum(&base_path, &ast);
    let generated_cqrs_fns = generate_cqrs_impl(
        &domain_model_struct_ident,
        &effect_ident,
        &effect_variants,
        &error_ident,
        &ast,
    );

    let generated_code = quote! {
        #generated_error_enum
        #generated_effect_enum
        #generated_cqrs_fns
    };
    debug!(
        "generated code:\n----------------------------------------------------------------------------------------\n{:}\n----------------------------------------------------------------------------------------\n",
        generated_code
    );
    Ok(generated_code)
}

#[cfg(test)]
mod tests {
    use crate::{
        generate_api_macro_impl::generate_code, utils::read_rust_files::read_rust_file_content,
    };
    use quote::quote;

    // use syn::{
    //     parse::{Parse, Parser},
    //     parse_str, File,
    // };
    // use proc_macro2::TokenStream;
    // pub fn prettyprint(tokens: TokenStream) -> String {
    //     prettyplease::unparse(&syn::File::parse.parse2(tokens).unwrap())
    // }

    #[test]
    fn generate_all_from_good_file_test() {
        let expected = quote! {
            use crate::good_source_file::MyGoodProcessingError;
            #[derive(thiserror :: Error, Debug)]
            pub enum ProcessingError {
                #[error("Error during processing: {0}")]
                MyGoodProcessingError(MyGoodProcessingError),
                #[error("Processing was fine, but state could not be persisted: {0}")]
                NotPersisted(#[source] std::io::Error),
            }
            pub enum Effect {
                MyGoodDomainModelRenderItems(RustAutoOpaque<MyGoodDomainModel>)
            }
            pub enum Cqrs {
                MyGoodDomainModelAddItem(String),
                MyGoodDomainModelRemoveItem(usize),
                MyGoodDomainModelCleanList,
                MyGoodDomainModelGetAllItems
            }
            impl Cqrs {
                pub(crate) fn process_with_app_state(
                    self,
                    app_state: &AppState,
                ) -> Result<Vec<Effect>, ProcessingError> {
                    let result = match self {
                        Cqrs::MyGoodDomainModelAddItem(item) => MyGoodDomainModel::add_item(app_state, item),
                        Cqrs::MyGoodDomainModelRemoveItem(todo_pos) => MyGoodDomainModel::remove_item(app_state, todo_pos),
                        Cqrs::MyGoodDomainModelCleanList => MyGoodDomainModel::clean_list(app_state),
                        Cqrs::MyGoodDomainModelGetAllItems => MyGoodDomainModel::get_all_items(app_state),
                    }
                    .map_err(ProcessingError::MyGoodProcessingError)?
                    .into_iter()
                    .map(|effect| match effect {
                        MyGoodDomainModelEffect::RenderItems(rust_auto_opaque_my_good_domain_model) =>
                            Effect::MyGoodDomainModelRenderItems(rust_auto_opaque_my_good_domain_model) ,
                    })
                    .collect();
                    Ok(result)
                }
                pub fn process(self) -> Result<Vec<Effect>, ProcessingError> {
                    let app_state = &Lifecycle::get().app_state;
                    let result = self.process_with_app_state(app_state)?;
                    let _ = app_state.persist().map_err(ProcessingError::NotPersisted);
                    Ok(result)
                }
            }
        };
        let (base_path, content) = read_rust_file_content("../tests/good_source_file/mod.rs")
            .expect("Could not read test oracle file: ");

        let result = generate_code(base_path, content).unwrap();
        assert_eq!(expected.to_string(), result.to_string());
    }
}
