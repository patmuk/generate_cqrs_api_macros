mod good_source_file;
use generate_cqrs_api_macros::generate_api;
use good_source_file::{AppStateImpl, MyGoodDomainModelLock};

pub struct LifecycleImpl {
    app_state: AppStateImpl,
}

#[generate_api("tests/good_source_file/mod.rs")]
impl Lifecycle for LifecycleImpl {
    fn new(_: std::option::Option<std::string::String>) -> &'static Self {
        unimplemented!()
    }
    fn get_singleton() -> &'static Self {
        unimplemented!()
    }
    fn app_config(&self) -> &impl AppConfig {
        let neverused = Box::new(AppConfigImpl::default());
        Box::leak(neverused)
    }
    fn app_state(&self) -> &impl AppState {
        let neverused = Box::new(AppStateImpl {
            my_good_domain_model_lock: MyGoodDomainModelLock::default(),
        });
        Box::leak(neverused)
    }
    fn persist(&self) -> std::result::Result<(), std::io::Error> {
        unimplemented!()
    }
    fn shutdown(&self) -> std::result::Result<(), std::io::Error> {
        unimplemented!()
    }
}
// impl Lifecycle for UiTests {}
