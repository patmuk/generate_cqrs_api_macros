use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn generate_cqrs_traits() -> TokenStream {
    quote! {
        pub(crate) trait CqrsModel:
            std::marker::Sized + Default + serde::Serialize + for<'de> serde::Deserialize<'de>{}
        pub(crate) trait CqrsModelLock<CqrsModel>:
            std::marker::Sized + Clone + serde::Serialize + for<'de> serde::Deserialize<'de>{
                fn for_model(model: CqrsModel) -> Self;
        }
        pub trait Cqrs: std::fmt::Debug {
            fn process(self) -> Result<Vec<Effect>, ProcessingError>;
        }
    }
}
