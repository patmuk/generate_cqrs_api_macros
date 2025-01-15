#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct AppStateImpl {
    pub my_good_domain_model_lock: MyGoodDomainModelLock,
    pub my_second_domain_model_lock: MySecondDomainModelLock,
}

pub(crate) type StateChanged = bool;

impl AppState for AppStateImpl {
    fn new<A: AppConfig>(app_config: &A) -> Self {
        let _ = app_config;
        todo!()
    }
    fn dirty_flag_value(&self) -> bool {
        todo!()
    }
    fn mark_dirty(&self) {
        todo!()
    }
}
