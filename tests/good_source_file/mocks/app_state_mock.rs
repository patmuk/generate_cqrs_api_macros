pub use crate::AppState;

pub(crate) struct AppStateImpl {
    pub my_good_domain_model_lock: MyGoodDomainModelLock,
}
impl AppStateImpl {
    pub(crate) fn mark_dirty(&self) {}
}

pub(crate) type StateChanged = bool;

impl AppState for AppStateImpl {
    fn load_or_new<A: AppConfig>(app_config: &A) -> Self {
        let _ = app_config;
        todo!()
    }
    fn persist_to_path(&self, _: &std::path::PathBuf) -> std::result::Result<(), std::io::Error> {
        todo!()
    }
    fn dirty_flag_value(&self) -> bool {
        todo!()
    }
    fn mark_dirty(&self) {
        todo!()
    }
}
