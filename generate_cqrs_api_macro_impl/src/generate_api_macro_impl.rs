use log::debug;

use crate::generating::generate_cqrs_impl::generate_cqrs_impl;
use crate::generating::generate_effects_enum::generate_effects_enum;
use crate::generating::generate_errors_enum::generate_errors_enum;
use crate::generating::generate_use_statement::generate_use_statement;
use crate::generating::traits::api_traits::generate_api_traits;
use crate::generating::traits::cqrs_traits::generate_cqrs_traits;

use crate::parsing::get_struct_by_trait::get_structs_by_traits;
// use crate::parsing::get_use_statements::get_use_statements;
use crate::parsing::read_rust_files::{read_rust_file_content, tokens_2_file_locations};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Result, Variant};

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct BasePath(pub(crate) String);
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct SourceCodeString(pub(crate) String);

pub(crate) struct ParsedFiles {
    pub(crate) base_path: BasePath,
    pub(crate) source_code: SourceCodeString,
}
pub(crate) struct ModelParsed {
    pub(crate) base_path: BasePath,
    pub(crate) ast: syn::File,
    pub(crate) domain_model_ident: Ident,
    pub(crate) domain_model_lock_ident: Ident,
}

pub(crate) struct ModelNEffects {
    pub(crate) base_path: BasePath,
    pub(crate) ast: syn::File,
    pub(crate) domain_model_ident: Ident,
    pub(crate) domain_model_lock_ident: Ident,
    pub(crate) effect_ident: Ident,
    pub(crate) effect_variants: Vec<Variant>,
}
pub(crate) struct ModelNEffectsNErrors {
    pub(crate) base_path: BasePath,
    pub(crate) ast: syn::File,
    pub(crate) domain_model_ident: Ident,
    pub(crate) domain_model_lock_ident: Ident,
    pub(crate) effect_ident: Ident,
    pub(crate) effect_variants: Vec<Variant>,
    pub(crate) error_ident: Ident,
}

pub fn generate_api_impl(item: TokenStream, file_paths: TokenStream) -> Result<TokenStream> {
    log::info!("-------- Generating API --------");
    // check if it implements the Lifecycle trait
    // not parsing with syn::parse, to save time. Returning the unchanged input anyways, would need to clone() otherwise
    if !item.to_string().contains("Lifecycle for ") {
        panic!("The macro has to be declaired on an 'impl Lifecycle for'!");
    }

    let file_locations = tokens_2_file_locations(file_paths)?;
    if file_locations.is_empty() {
        panic!("At least one model implementatoin struct has to be provided\nlike #[generate_api(\"domain/MyModel.rs\")]\nProvide multiple model implementations with #[generate_api(\"domain/MyModel.rs\", \"other_domain/MySecondModel.rs\")]");
    }
    let parsed_files = read_rust_file_content(file_locations)?;

    let generated_code = generate_code(parsed_files)?;

    let output = quote! {
        #item
        #generated_code
    };
    Ok(output)
}

fn generate_code(parsed_files: Vec<ParsedFiles>) -> Result<TokenStream> {
    let models_parsed: Vec<ModelParsed> = parsed_files.into_iter().map(|parsed_file|{
        let ast = syn::parse_file(&parsed_file.source_code.0).unwrap_or_else(|_| panic!("cannot parse the code file {}", parsed_file.base_path.0));
        let trait_impls = get_structs_by_traits(&ast, &["CqrsModel", "CqrsModelLock"]);
        let domain_model_ident = trait_impls
            .get("CqrsModel")
            .expect("Couldn't extract the domain model's name. One Struct needs to derive CqrsModel.");
        debug!("domain model name: {:#?}", domain_model_ident);
        let domain_model_lock_ident = trait_impls.get("CqrsModelLock").expect(
            "Couldn't extract the domaModelel lock's name. One Struct needs to derive CqrsModelLock.",
        );
        debug!("domain model lock name: {:#?}", domain_model_lock_ident);
        ModelParsed{base_path : parsed_file.base_path, ast, domain_model_ident: domain_model_ident.to_owned(), domain_model_lock_ident: domain_model_lock_ident.to_owned() }
    }).collect();
    // take all imports, just in case they are used in the generated code (like RustAutoOpaque)
    // => not needed. If needed later, remove import to generated traits!
    // let use_statements = get_use_statements(&ast);

    let (models_n_effect, generated_effect_enum) = generate_effects_enum(models_parsed);
    let (models_n_efects_n_errors, generated_error_enum) = generate_errors_enum(models_n_effect);
    let generated_cqrs_fns = &generate_cqrs_impl(&models_n_efects_n_errors);
    let generated_api_traits = generate_api_traits();
    let generated_cqrs_traits = generate_cqrs_traits();

    let use_statements = models_n_efects_n_errors
        .iter()
        .map(|model_n_effects_n_errors: &ModelNEffectsNErrors| {
            generate_use_statement(&model_n_effects_n_errors.base_path, "*")
        })
        .collect::<Vec<TokenStream>>();

    let generated_code = quote! {
        #(#use_statements)*
        #generated_api_traits
        #generated_cqrs_traits
        #generated_error_enum
        #generated_effect_enum
        #(#generated_cqrs_fns)*
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
            use crate::good_source_file::MyGoodProcessingError;
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
        // let (use_statements, content) =
        let paths_n_codes =
            read_rust_file_content(vec!["../tests/good_source_file/mod.rs".to_string()])
                .expect("Could not read test oracle file: ");
        let result = generate_code(paths_n_codes).unwrap();
        assert_eq!(expected.to_string(), result.to_string());
    }

    #[test]
    fn generate_all_from_two_files_test() {
        let expected = quote! {
                    use crate::good_source_file::*;
                    use crate::second_model_file::*;
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
                    use crate::good_source_file::MyGoodProcessingError;
                    use crate::second_model_file::MySecondDomainProcessingError;
                    #[derive(thiserror :: Error, Debug)]
                    pub enum ProcessingError {
                        #[error("Error during processing: {0}")]
                        MyGoodProcessingError(MyGoodProcessingError),
                        #[error("Error during processing: {0}")]
                        MySecondDomainProcessingError(MySecondDomainProcessingError),
                        #[error("Processing was fine, but state could not be persisted: {0}")]
                        NotPersisted(#[source] std::io::Error),
                    }
                    pub enum Effect {
                        MyGoodDomainModelRenderItems(MyGoodDomainModelLock),
                        MySecondDomainModelRenderItems(MySecondDomainModelLock),
                        MySecondDomainModelAlert
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
                    #[derive(Debug)]
        pub enum MySecondDomainModelQuery {
            GetAllItems
        }
        #[derive(Debug)]
        pub enum MySecondDomainModelCommand {
            AddSecondItem(String),
            CleanList,
            ReplaceItem(usize)
        }
        impl Cqrs for MySecondDomainModelQuery {
            fn process(self) -> Result<Vec<Effect>, ProcessingError> {
                self.process_with_lifecycle(LifecycleImpl::get_singleton())
            }
        }
        impl MySecondDomainModelQuery {
            fn process_with_lifecycle(
                self,
                lifecycle: &LifecycleImpl,
            ) -> Result<Vec<Effect>, ProcessingError> {
                let app_state = &lifecycle.app_state;
                let my_second_domain_model_lock = &app_state.my_second_domain_model_lock;
                let result = match self {
                    MySecondDomainModelQuery::GetAllItems => my_second_domain_model_lock.get_all_items(),
                }
                .map_err(ProcessingError::MySecondDomainProcessingError)?;
                Ok(result
                    .into_iter()
                    .map(|effect| match effect {
                        MySecondDomainModelEffect::RenderItems(my_second_domain_model_lock) =>
                        Effect::MySecondDomainModelRenderItems(my_second_domain_model_lock),
                        MySecondDomainModelEffect::Alert => Effect::MySecondDomainModelAlert,
                    })
                    .collect())
            }
        }
        impl Cqrs for MySecondDomainModelCommand {
            fn process(self) -> Result<Vec<Effect>, ProcessingError> {
                self.process_with_lifecycle(LifecycleImpl::get_singleton())
            }
        }
        impl MySecondDomainModelCommand {
            fn process_with_lifecycle(
                self,
                lifecycle: &LifecycleImpl,
            ) -> Result<Vec<Effect>, ProcessingError> {
                let app_state = &lifecycle.app_state;
                let my_second_domain_model_lock = &app_state.my_second_domain_model_lock;
                let (state_changed, result) = match self {
                    MySecondDomainModelCommand::AddSecondItem(item) => my_second_domain_model_lock.add_second_item(item),
                    MySecondDomainModelCommand::CleanList => my_second_domain_model_lock.clean_list(),
                    MySecondDomainModelCommand::ReplaceItem(todo_pos) => my_second_domain_model_lock.replace_item(todo_pos),
                }
                .map_err(ProcessingError::MySecondDomainProcessingError)?;
                if state_changed {
                    app_state.mark_dirty();
                    lifecycle.persist().map_err(ProcessingError::NotPersisted)?;
                }
                Ok(result
                    .into_iter()
                    .map(|effect| match effect {
                        MySecondDomainModelEffect::RenderItems(my_second_domain_model_lock) => Effect::MySecondDomainModelRenderItems(my_second_domain_model_lock),
                        MySecondDomainModelEffect::Alert => Effect::MySecondDomainModelAlert,
                    })
                    .collect())
            }
        }
        };
        // let (use_statements, content) =
        let paths_n_codes = read_rust_file_content(vec![
            "../tests/good_source_file/mod.rs".to_string(),
            "../tests/second_model_file/mod.rs".to_string(),
        ])
        .expect("Could not read test oracle file: ");
        let result = generate_code(paths_n_codes).unwrap();
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
    #[should_panic(expected = "The macro has to be declaired on an 'impl Lifecycle for'!")]
    fn test_gengenerate_api_impl_wrong_lifecycle_impl() {
        let lifecycle_not_trait_impl = quote! {
            impl Lifecycle {}
        };
        let _ = generate_api_impl(lifecycle_not_trait_impl, quote! {});
    }
}
