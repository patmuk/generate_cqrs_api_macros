mod good_source_file;
mod second_model_file;

use generate_cqrs_api_macro::generate_api;
// use good_source_file::{AppStateImpl, MyGoodDomainModelLock};

include!("./mocks/app_config_mock.rs");
include!("./mocks/app_state_mock.rs");
include!("./mocks/app_state_persister_mock.rs");
include!("./mocks/rust_auto_opaque_mock.rs");

pub struct LifecycleImpl {
    app_state: AppStateImpl,
    persister: AppStatePersisterMock,
}

#[generate_api("tests/good_source_file/mod.rs", "tests/second_model_file/mod.rs")]
// #[generate_api("tests/second_model_file/mod.rs")]
// #[generate_api("tests/good_source_file/mod.rs")]
impl Lifecycle for LifecycleImpl {
    type Error = AppStatePersisterErrorMock;
    fn initialise_with_app_config<AC: AppConfig + std::fmt::Debug>(
        _app_config: AC,
    ) -> Result<&'static Self, Self::Error> {
        unimplemented!()
    }
    fn initialise(_app_state_url: Option<String>) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn get_singleton() -> &'static Self {
        unimplemented!()
    }
    /// persist the app state to the previously stored location
    fn persist() -> Result<(), ProcessingError> {
        let lifecycle = Self::get_singleton();
        lifecycle
            .persister
            .persist_app_state(&lifecycle.app_state)
            .map_err(|err| err.to_processing_error())
    }

    fn shutdown() -> Result<(), ProcessingError> {
        // blocks on the Locks of inner fields
        // TODO implent timeout and throw an error?
        Self::persist()
    }
}
