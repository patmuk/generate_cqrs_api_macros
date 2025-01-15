use crate::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MyGoodDomainModel {
    items: Vec<DomainItem>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct DomainItem {
    text: String,
}

#[derive(Debug, Clone, Default)]
pub struct MyGoodDomainModelLock {
    pub(crate) lock: RustAutoOpaque<MyGoodDomainModel>,
}

impl Serialize for MyGoodDomainModelLock {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize the model , the dirty flag is always false after loading
        self.lock.blocking_read().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MyGoodDomainModelLock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let list_model = MyGoodDomainModel::deserialize(deserializer)?;
        Ok(Self::for_model(list_model))
    }
}

impl CqrsModelLock<MyGoodDomainModel> for MyGoodDomainModelLock {
    fn for_model(_model: MyGoodDomainModel) -> Self {
        todo!()
    }
}

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
