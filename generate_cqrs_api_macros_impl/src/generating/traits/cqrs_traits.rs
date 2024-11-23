use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn generate_cqrs_traits() -> TokenStream {
    quote! {
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
    }
}
