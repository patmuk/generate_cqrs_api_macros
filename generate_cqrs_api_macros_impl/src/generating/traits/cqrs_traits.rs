use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn generate_cqrs_traits() -> TokenStream {
    quote! {
        use std::fmt::Debug;
        // use crate::application::api::lifecycle::{Effect, ProcessingError};

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

        pub trait Cqrs: Debug {
            // type Effect;
            // type ProcessingError;
            // fn process(self) -> Result<Vec<Self::Effect>, Self::ProcessingError>;
            fn process(self) -> Result<Vec<Effect>, ProcessingError>;
        }
    }
}
