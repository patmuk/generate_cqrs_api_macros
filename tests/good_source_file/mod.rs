use crate::{CqrsModel, CqrsModelLock};

include!("./mocks/app_config_mock.rs");
include!("./mocks/app_state_mock.rs");
include!("./mocks/rust_auto_opaque_mock.rs");

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MyGoodDomainModel {
    items: Vec<DomainItem>,
}

#[derive(Clone, Debug, PartialEq)]
struct DomainItem {
    text: String,
}

#[derive(Debug, Clone, Default)]
pub struct MyGoodDomainModelLock {
    pub(crate) lock: RustAutoOpaque<MyGoodDomainModel>,
}
impl CqrsModelLock<MyGoodDomainModel> for MyGoodDomainModelLock {}

#[allow(dead_code)]
pub enum MyGoodDomainModelEffect {
    RenderItems(MyGoodDomainModelLock),
}

#[allow(dead_code)]
impl MyGoodDomainModel {
    pub fn get_items_as_string(&self) -> Vec<String> {
        self.items.iter().map(|item| item.text.clone()).collect()
    }
}

impl From<MyGoodDomainModel> for MyGoodDomainModelLock {
    fn from(model: MyGoodDomainModel) -> Self {
        MyGoodDomainModelLock {
            lock: RustAutoOpaque::new(model),
        }
    }
}

impl From<MyGoodDomainModelLock> for MyGoodDomainModel {
    fn from(val: MyGoodDomainModelLock) -> Self {
        let _ = val;
        todo!();
        // this should work - maybe it doesn't because of the RustAutoOpaque mock implementation
        // std::mem::take(&mut *val.lock.blocking_write())
    }
}

#[allow(dead_code)]
impl MyGoodDomainModelLock {
    pub(crate) fn add_item(
        &self,
        item: String,
    ) -> Result<(bool, Vec<MyGoodDomainModelEffect>), MyGoodProcessingError> {
        self.lock
            .blocking_write()
            .items
            .push(DomainItem { text: item });
        // this clone is cheap, as it is on ARC (RustAutoOpaque>T> = Arc<RwMutex<T>>)
        Ok((
            true,
            vec![MyGoodDomainModelEffect::RenderItems(self.clone())],
        ))
    }
    pub(crate) fn remove_item(
        &self,
        todo_pos: usize,
    ) -> Result<(bool, Vec<MyGoodDomainModelEffect>), MyGoodProcessingError> {
        let items = &mut self.lock.blocking_write().items;
        if todo_pos > items.len() {
            Err(MyGoodProcessingError::ItemDoesNotExist(todo_pos))
        } else {
            items.remove(todo_pos - 1);
            Ok((
                true,
                vec![MyGoodDomainModelEffect::RenderItems(self.clone())],
            ))
        }
    }
    pub(crate) fn clean_list(
        &self,
    ) -> Result<(bool, Vec<MyGoodDomainModelEffect>), MyGoodProcessingError> {
        self.lock.blocking_write().items.clear();
        Ok((
            true,
            vec![MyGoodDomainModelEffect::RenderItems(self.clone())],
        ))
    }
    pub(crate) fn get_all_items(
        &self,
    ) -> Result<Vec<MyGoodDomainModelEffect>, MyGoodProcessingError> {
        Ok(vec![MyGoodDomainModelEffect::RenderItems(self.clone())])
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum MyGoodProcessingError {
    #[error("The todo at index {0} does not exist!")]
    ItemDoesNotExist(usize),
}

impl CqrsModel for MyGoodDomainModel {}
