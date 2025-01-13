use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn generate_api_traits() -> TokenStream {
    quote! {
        pub trait Lifecycle {
            /// loads the app's state, which can be io-heavy
            /// get the instance with get_singleton(). Create the initial singleton with UnInitilizedLifecycle::init()
            fn new<
                AC: AppConfig + std::fmt::Debug,
                ASP: AppStatePersister,
                ASPE: AppStatePersistError + From<(std::io::Error, String)> + From<(bincode::Error, String)>,
            >(
                app_config: AC,
            ) -> Result<&'static Self, ASPE>;
            /// get the instance with get_singleton(). Create the initial singleton with Lifecycle::new()
            fn get_singleton() -> &'static Self;
            fn borrow_app_config<AC: AppConfig>(&self) -> &AC;
            fn borrow_app_state<AS: AppState>(&self) -> &AS;
            /// persist the app state to the previously stored location
            fn persist(&self) -> Result<(), ProcessingError>;
            fn shutdown(&self) -> Result<(), ProcessingError>;
        }
        pub trait AppConfig: Default {
            /// call to overwrite default values.
            /// Doesn't trigger long initialization operations.
            fn new(url: Option<String>) -> Self;
            // app state storage location
            fn get_app_state_url(&self) -> &str;
        }

        pub trait AppState {
            fn new<AC: AppConfig>(app_config: &AC) -> Self;
            fn dirty_flag_value(&self) -> bool;
            fn mark_dirty(&self);
        }

        pub trait AppStatePersistError: std::error::Error {
            /// convert to ProcessingError::NotPersisted
            fn to_processing_error(&self) -> ProcessingError;
        }

        pub trait AppStatePersister {
            /// prepares for persisting a new AppState. Not needed if the AppState is loaded!
            fn new<AC: AppConfig, ASPE: AppStatePersistError + From<(std::io::Error, String)>>(
                app_config: &AC,
            ) -> Result<Self, ASPE>
            where
                Self: Sized;
            /// Loads the application state.
            /// Returns a result with the `AppState` if successful or an `InfrastructureError` otherwise.
            fn load_app_state<
                AC: AppConfig,
                AS: AppState + Serialize + for<'a> Deserialize<'a>,
                ASPE: AppStatePersistError + From<(std::io::Error, String)> + From<(bincode::Error, String)>,
            >(
                &self,
            ) -> Result<AS, ASPE>;

            /// Persists the application state to storage.
            /// Ensures that the `AppState` is stored in a durable way, regardless of the underlying mechanism.
            fn persist_app_state<
                AS: AppState + Serialize + for<'a> Deserialize<'a> + std::fmt::Debug,
                ASPE: AppStatePersistError + From<(std::io::Error, String)>,
            >(
                &self,
                state: &AS,
            ) -> Result<(), ASPE>;
        }
    }
}
