use log::debug;

use crate::generating::generate_cqrs_impl::generate_cqrs_impl;
use crate::generating::generate_effect_enum::generate_effect_enum;
use crate::generating::generate_error_enum::generate_error_enum;
use crate::generating::generate_use_statement::generate_use_statement;
use crate::generating::traits::api_traits::generate_api_traits;
use crate::generating::traits::cqrs_traits::generate_cqrs_traits;
use crate::parsing::get_domain_model_struct::get_domain_model_struct_ident;
// use crate::parsing::get_use_statements::get_use_statements;
use crate::parsing::read_rust_files::{read_rust_file_content, tokens_2_file_locations};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

pub(crate) struct BasePath(pub(crate) String);
pub(crate) struct SourceCode(pub(crate) String);

pub fn generate_api_impl(item: TokenStream, file_pathes: TokenStream) -> Result<TokenStream> {
    log::info!("-------- Generating API --------");
    // check if it implements the Lifecycle trait
    // not parsing with syn::parse, to save time. Returning the unchanged input anyways, would need to clone() otherwise
    if !item.to_string().contains("Lifecycle for ") {
        panic!("The macro has to be declaired on an 'impl api_traits::Lifecycle for'!");
    }

    let file_locations = tokens_2_file_locations(file_pathes)?;
    if file_locations.is_empty() {
        panic!("At least one model implementatoin struct has to be provided\nlike #[generate_api(\"domain/MyModel.rs\")]");
    }
    let (base_path, file_content) = read_rust_file_content(&file_locations[0])?;

    let generated_code = generate_code(base_path, file_content)?;

    let output = quote! {
        #item
        #generated_code
    };
    Ok(output)
}

fn generate_code(base_path: BasePath, file_content: SourceCode) -> Result<TokenStream> {
    let ast = syn::parse_file(&file_content.0)?;
    // take all imports, just in case they are used in the generated code (like RustAutoOpaque)
    // not needed. If needed later, remove import to generated traits!
    // let use_statements = get_use_statements(&ast);
    // import all types defined in the parsed file
    let base_use_statement = generate_use_statement(&base_path, "*");

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
    let generated_api_traits = generate_api_traits();
    let generated_cqrs_traits = generate_cqrs_traits();

    let generated_code = quote! {
        #base_use_statement
        // #(#use_statements)*
        #generated_api_traits
        #generated_cqrs_traits
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
        generate_api_macro_impl::generate_code, parsing::read_rust_files::read_rust_file_content,
    };
    use quote::quote;

    use super::generate_api_impl;

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
            use crate::good_source_file::*;
            use log::debug;
            use std::path::PathBuf;
            pub trait Lifecycle {
                #[doc = r" the app config is to be set only once, and read afterwards. If mutation is needed wrapp it into a lock for concurrent write access"]
                #[doc = r" to avoid an illegal state (app state not loaded) we do the setup and init in one go"]
                #[doc = r" get the instance with get()"]
                fn new(path: Option<String>) -> &'static Self;
                fn get_singleton() -> &'static Self;
                fn app_config(&self) -> &impl AppConfig;
                fn app_state(&self) -> &impl AppState;
                #[doc = r" call to initialize the app."]
                #[doc = r" loads the app's state, which can be io-heavy"]
                fn init<AC: AppConfig, AS: AppState>(app_config: &AC) -> AS {
                    debug!("Initializing app with config: {:?}", app_config);
                    AppState::load_or_new(app_config)
                }
                fn persist(&self) -> Result<(), std::io::Error>;
                fn shutdown(&self) -> Result<(), std::io::Error>;
            }
            pub trait AppConfig: Default + std::fmt::Debug {
                #[doc = r" call to overwrite default values."]
                #[doc = r" Doesn't trigger initialization."]
                fn new(path: Option<String>) -> Self;
                fn get_app_state_file_path(&self) -> &std::path::PathBuf;
            }
            pub trait AppState {
                fn load_or_new<A: AppConfig>(app_config: &A) -> Self
                where
                    Self: Sized;
                #[allow(clippy::ptr_arg)]
                fn persist_to_path(&self, path: &PathBuf) -> Result<(), std::io::Error>;
                fn dirty_flag_value(&self) -> bool;
                fn mark_dirty(&self);
            }
            pub(crate) trait CqrsModel: std::marker::Sized + Default {
                fn new() -> Self {
                    Self::default()
                }
            }
            pub(crate) trait CqrsModelLock<CqrsModel>:
                Default + From<CqrsModel> + std::marker::Sized + Clone
            {
                fn new() -> Self {
                    Self::default()
                }
            }
            pub trait Cqrs: std::fmt::Debug {
                fn process(self) -> Result<Vec<Effect>, ProcessingError>;
            }
            use crate :: good_source_file :: MyGoodProcessingError ;

            #[derive(thiserror :: Error, Debug)]
            pub enum ProcessingError {
                #[error("Error during processing: {0}")]
                MyGoodProcessingError(MyGoodProcessingError),
                #[error("Processing was fine, but state could not be persisted: {0}")]
                NotPersisted(#[source] std::io::Error),
            }
            pub enum Effect {
                MyGoodDomainModelRenderItems(MyGoodDomainModelLock)
            }
            #[derive(Debug)]
            pub enum MyGoodDomainModelQuery {
                GetAllItems
            }
            #[derive(Debug)]
            pub enum MyGoodDomainModelCommand {
                AddItem(String),
                CleanList,
                RemoveItem(usize)
            }

            impl Cqrs for MyGoodDomainModelQuery {
                fn process(self) -> Result<Vec<Effect>, ProcessingError> {
                    self.process_with_lifecycle(LifecycleImpl::get_singleton())
                }
            }
            impl MyGoodDomainModelQuery {
                fn process_with_lifecycle(
                    self,
                    lifecycle: &LifecycleImpl,
                ) -> Result<Vec<Effect>, ProcessingError> {
                    let app_state = &lifecycle.app_state;
                    let my_good_domain_model_lock = &app_state.my_good_domain_model_lock;
                    let result = match self {
                        MyGoodDomainModelQuery::GetAllItems => my_good_domain_model_lock.get_all_items(),
                    }
                    .map_err(ProcessingError::MyGoodProcessingError)?;
                    Ok(result
                    .into_iter()
                    .map(|effect| match effect {
                        MyGoodDomainModelEffect::RenderItems(my_good_domain_model_lock) =>
                            Effect::MyGoodDomainModelRenderItems(my_good_domain_model_lock) ,
                    })
                    .collect())
                }
            }
            impl Cqrs for MyGoodDomainModelCommand {
                fn process(self) -> Result<Vec<Effect>, ProcessingError> {
                    self.process_with_lifecycle(LifecycleImpl::get_singleton())
                }
            }
            impl MyGoodDomainModelCommand {
                fn process_with_lifecycle(
                    self,
                    lifecycle: &LifecycleImpl,
                ) -> Result<Vec<Effect>, ProcessingError> {
                    let app_state = &lifecycle.app_state;
                    let my_good_domain_model_lock = &app_state.my_good_domain_model_lock;
                    let (state_changed, result) = match self {
                        MyGoodDomainModelCommand::AddItem(item) => my_good_domain_model_lock.add_item(item),
                        MyGoodDomainModelCommand::CleanList => my_good_domain_model_lock.clean_list(),
                        MyGoodDomainModelCommand::RemoveItem(todo_pos) =>
                            my_good_domain_model_lock.remove_item(todo_pos),
                    }
                    .map_err(ProcessingError::MyGoodProcessingError)?;
                    if state_changed {
                        app_state.mark_dirty();
                        lifecycle.persist().map_err(ProcessingError::NotPersisted)?;
                    }
                    Ok(result
                        .into_iter()
                        .map(|effect| match effect {
                            MyGoodDomainModelEffect::RenderItems(my_good_domain_model_lock) =>
                                Effect::MyGoodDomainModelRenderItems(my_good_domain_model_lock),
                        })
                        .collect())
                }
            }
        };
        let (base_path, content) = read_rust_file_content("../tests/good_source_file/mod.rs")
            .expect("Could not read test oracle file: ");

        let result = generate_code(base_path, content).unwrap();
        assert_eq!(expected.to_string(), result.to_string());
    }

    #[test]
    #[should_panic(
        expected = "At least one model implementatoin struct has to be provided\nlike #[generate_api(\"domain/MyModel.rs\")]"
    )]
    fn test_gengenerate_api_impl_no_model_struct() {
        let lifecycle_impl = quote! {
            impl api_traits::Lifecycle for Lifecycle {}
        };
        let _ = generate_api_impl(lifecycle_impl, quote! {});
    }
    #[test]
    #[should_panic(
        expected = "The macro has to be declaired on an 'impl api_traits::Lifecycle for'!"
    )]
    fn test_gengenerate_api_impl_wrong_lifecycle_impl() {
        let lifecycle_not_trait_impl = quote! {
            impl Lifecycle {}
        };
        let _ = generate_api_impl(lifecycle_not_trait_impl, quote! {});
    }
}
