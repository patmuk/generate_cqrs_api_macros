use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn generate_api_traits() -> TokenStream {
    quote! {
        use serde::{Deserialize, Serialize};

        pub trait Lifecycle {
            /// due to frb's current capabilities we cannot define function arguments as types.
            /// for return types it works. Thus, Error is defined this way, while AppConfig is a generic parameter.
            type Error: AppStatePersistError;
            /// loads the app's state, which can be io-heavy
            /// get the instance with get_singleton(). Create the initial singleton with this function
            fn initialise<AC: AppConfig + std::fmt::Debug>(
                app_config: AC,
            ) -> Result<&'static Self, Self::Error>;

            /// frb doesn't support generics. Thus, we can call this concrete function.
            fn initialise_with_file_persister(app_config: AppConfigImpl) -> Result<(), Self::Error>;

            /// get the instance with get_singleton(). Create the initial singleton with Lifecycle::initialise()
            /// This cannot be called from Flutter, as frb cannot handle references. Thus, it is called internally (by CQRS::process(), Lifecycle::shutdown() and others)
            fn get_singleton() -> &'static Self;
            /// persist the app state to the previously stored location
            /// as we cannot pass references to frb (see 'get_singleton') persist() and shutdown() have to get 'self' by calling get_singleton() on their own.
            fn persist() -> Result<(), ProcessingError>;
            fn shutdown() -> Result<(), ProcessingError>;
        }

        pub trait AppConfig: Default {
            /// call to overwrite default values.
            /// Doesn't trigger long initialization operations.
            fn new(url: Option<String>) -> Self;
            /// app state storage location
            fn borrow_app_state_url(&self) -> &str;
        }

        /// the app's state is not exposed external - it is guarded behind CQRS functions
        pub(crate) trait AppState: Serialize + for<'a> Deserialize<'a> {
            fn new<AC: AppConfig>(app_config: &AC) -> Self;
            fn dirty_flag_value(&self) -> bool;
            fn mark_dirty(&self);
        }

        pub trait AppStatePersistError:
            std::error::Error
        {
            /// convert to ProcessingError::NotPersisted
            fn to_processing_error(&self) -> ProcessingError;
        }

        pub(crate) trait AppStatePersister {
            /// prepares for persisting a new AppState. Not needed if the AppState is loaded!
            type Error: AppStatePersistError;
            fn new<AC: AppConfig>(app_config: &AC) -> Result<Self, Self::Error>
            where
                Self: Sized;
            /// Persists the application state to storage.
            /// Ensures that the `AppState` is stored in a durable way, regardless of the underlying mechanism.
            fn persist_app_state<AS: AppState + std::fmt::Debug>(
                &self,
                state: &AS,
            ) -> Result<(), Self::Error>;

            /// Loads the application state.
            /// Returns a result with the `AppState` if successful or an `InfrastructureError` otherwise.
            fn load_app_state<AC: AppConfig, AS: AppState>(&self) -> Result<AS, Self::Error>;
        }
    }
}
