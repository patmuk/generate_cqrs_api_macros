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
use crate::parsing::type_2_ident::get_ident;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, Ident, ImplItemType, ItemImpl, Result, Variant};

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
    if !item.to_string().contains("impl Lifecycle for") {
        panic!("The macro has to be declaired on an 'impl Lifecycle for'! (You can't use generics, as the singleton instance is to be stored as a static global variable.)");
    }
    let file_locations = tokens_2_file_locations(file_paths)?;
    if file_locations.is_empty() {
        panic!("At least one model implementatoin struct has to be provided\nlike #[generate_api(\"domain/MyModel.rs\")]\nProvide multiple model implementations with #[generate_api(\"domain/MyModel.rs\", \"other_domain/MySecondModel.rs\")]");
    }

    let lifecycle_impl_ident: Ident = get_type_ident_from_impl(&item)?;

    let parsed_files = read_rust_file_content(file_locations)?;

    let generated_code = generate_code(lifecycle_impl_ident, parsed_files)?;

    let output = quote! {
        #item
        #generated_code
    };
    Ok(output)
}

fn get_type_ident_from_impl(item: &TokenStream) -> Result<Ident> {
    let ast = parse2::<ItemImpl>(item.clone())?;
    // let tipe = get_structs_by_traits(&ast, ["Lifecycle"]);
    get_ident(&ast.self_ty)
}

fn generate_code(
    lifecycle_impl_ident: Ident,
    parsed_files: Vec<ParsedFiles>,
) -> Result<TokenStream> {
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
    let generated_cqrs_fns = &generate_cqrs_impl(&lifecycle_impl_ident, &models_n_efects_n_errors);
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
        generate_api_macro_impl::{generate_code, get_type_ident_from_impl},
        parsing::read_rust_files::read_rust_file_content,
    };
    use quote::{format_ident, quote};

    use super::generate_api_impl;

    #[test]
    fn parse_lifecycle_impl_ident() {
        let lifecycle_impl_token = quote! {
            impl Lifecycle for LifecycleImpl {
            type AppConfig = AppConfigImpl;
            type AppState = AppStateImpl;
            type AppStatePersister = AppStateFilePersister;

            fn new(
                app_config: Self::AppConfig,
                persister: Self::AppStatePersister,
            ) -> Result<&'static Self, AppStatePersistError> {
                // as this is static, it is executed one time only! So there is only one OnceLock instance.
                // static SINGLETON: OnceLock<Self::LifecycleSingleton> = OnceLock::new();
                static SINGLETON: OnceLock<LifecycleImpl> = OnceLock::new();

                info!("Initializing app with config: {:?}", &app_config);
                // calling init() the first time creates the singleton. (Although self is consumed, there migth be multiple instances of self.)
                // not using SINGLETON.get_or_init() so we can propergate the AppStatePersistError
                let result = match SINGLETON.get() {
                    Some(instance) => Ok(instance),
                    None => {
                        let app_state = match persister.load_app_state() {
                            Ok(app_state) => app_state,
                            Err(AppStatePersistError::DiskError(disk_err)) => match disk_err {
                                AppStateFilePersisterError::FileNotFound(file_path)
                                // todo match on IO-FileNotFound or avoid this error type duplication
                                // | AppStateFilePersisterError::IOError(io_Error, file_path)
                                //     if io_Error.kind() == IoErrorKind::NotFound
                                    =>
                                {
                                    info!(
                                        "No app state file found in {:?}, creating new state there.",
                                        &file_path
                                    );
                                    let app_state = Self::AppState::new(&app_config);
                                    persister.persist_app_state(&app_state)?;
                                    app_state
                                }
                                _ => return Err(AppStatePersistError::DiskError(disk_err)),
                            },
                            Err(e) => return Err(e),
                        };
                        // let lifecycle_singleton = LifecycleSingleton {
                        //     instance: LifecycleImpl {
                        //         app_config,
                        //         app_state,
                        //         persister,
                        //     },
                        // };
                        let lifecycle_singleton = LifecycleImpl {
                            app_config,
                            app_state,
                            persister,
                        };
                        SINGLETON.set(lifecycle_singleton);
                        Ok(SINGLETON
                            .get()
                            .expect("Impossible error - content has just been set!"))
                    }
                };
                info!(
                    "Initialization finished, log level is {:?}",
                    log::max_level()
                );
                result
            }

            fn get_singleton() -> &'static Self {
                SINGLETON
                    .get()
                    .expect("Lifecycle: should been initialized with UnInitializedLifecycle::init()!")
            }

            fn app_state(&self) -> &Self::AppState {
                &self.app_state
            }

            fn app_config(&self) -> &Self::AppConfig {
                &self.app_config
            }

            /// persist the app state to the previously stored location
            fn persist(&self) -> Result<(), AppStatePersistError> {
                self.persister.persist_app_state(&self.app_state)
            }

            fn shutdown(&self) -> Result<(), AppStatePersistError> {
                info!("shutting down the app");
                // blocks on the Locks of inner fields
                // TODO implent timeout and throw an error?
                self.persist()
            }
        }
        };

        assert_eq!(
            "LifecycleImpl".to_string(),
            get_type_ident_from_impl(&lifecycle_impl_token)
                .unwrap()
                .to_string()
        );
    }
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
                    let lifecycle: LifecycleImpl = Lifecycle::get_singleton();
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
                    let lifecycle: LifecycleImpl = Lifecycle::get_singleton();
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
        let result = generate_code(format_ident!("LifecycleImpl"), paths_n_codes).unwrap();
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
                            let lifecycle: LifecycleImpl = Lifecycle::get_singleton();
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
                            let lifecycle: LifecycleImpl = Lifecycle::get_singleton();
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
                let lifecycle: LifecycleImpl = Lifecycle::get_singleton();
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
                let lifecycle: LifecycleImpl = Lifecycle::get_singleton();
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
        let result = generate_code(format_ident!("LifecycleImpl"), paths_n_codes).unwrap();
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
