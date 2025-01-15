use crate::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MySecondDomainModel {
    items: Vec<SecondDomainItem>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct SecondDomainItem {
    text: String,
}

#[derive(Debug, Clone, Default)]
pub struct MySecondDomainModelLock {
    pub(crate) lock: RustAutoOpaque<MySecondDomainModel>,
}

impl Serialize for MySecondDomainModelLock {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize the model , the dirty flag is always false after loading
        self.lock.blocking_read().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MySecondDomainModelLock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let list_model = MySecondDomainModel::deserialize(deserializer)?;
        Ok(Self::for_model(list_model))
    }
}

impl CqrsModelLock<MySecondDomainModel> for MySecondDomainModelLock {
    fn for_model(_model: MySecondDomainModel) -> Self {
        todo!()
    }
}

#[allow(dead_code)]
pub enum MySecondDomainModelEffect {
    RenderItems(MySecondDomainModelLock),
    Alert,
}

#[allow(dead_code)]
impl MySecondDomainModel {
    pub fn get_items(&self) -> Vec<String> {
        self.items.iter().map(|item| item.text.clone()).collect()
    }
}

impl From<MySecondDomainModel> for MySecondDomainModelLock {
    fn from(model: MySecondDomainModel) -> Self {
        MySecondDomainModelLock {
            lock: RustAutoOpaque::new(model),
        }
    }
}

impl From<MySecondDomainModelLock> for MySecondDomainModel {
    fn from(val: MySecondDomainModelLock) -> Self {
        let _ = val;
        todo!();
        // this should work - maybe it doesn't because of the RustAutoOpaque mock implementation
        // std::mem::take(&mut *val.lock.blocking_write())
    }
}

#[allow(dead_code)]
impl MySecondDomainModelLock {
    pub(crate) fn add_second_item(
        &self,
        item: String,
    ) -> Result<(bool, Vec<MySecondDomainModelEffect>), MySecondDomainProcessingError> {
        self.lock
            .blocking_write()
            .items
            .push(SecondDomainItem { text: item });
        // this clone is cheap, as it is on ARC (RustAutoOpaque>T> = Arc<RwMutex<T>>)
        Ok((
            true,
            vec![MySecondDomainModelEffect::RenderItems(self.clone())],
        ))
    }
    pub(crate) fn replace_item(
        &self,
        todo_pos: usize,
    ) -> Result<(bool, Vec<MySecondDomainModelEffect>), MySecondDomainProcessingError> {
        let items = &mut self.lock.blocking_write().items;
        if todo_pos > items.len() {
            Err(MySecondDomainProcessingError::ItemDoesNotExist(todo_pos))
        } else {
            items.remove(todo_pos - 1);
            Ok((true, vec![MySecondDomainModelEffect::Alert]))
        }
    }
    pub(crate) fn clean_list(
        &self,
    ) -> Result<(bool, Vec<MySecondDomainModelEffect>), MySecondDomainProcessingError> {
        self.lock.blocking_write().items.clear();
        Ok((
            true,
            vec![MySecondDomainModelEffect::RenderItems(self.clone())],
        ))
    }
    pub(crate) fn get_all_items(
        &self,
    ) -> Result<Vec<MySecondDomainModelEffect>, MySecondDomainProcessingError> {
        Ok(vec![MySecondDomainModelEffect::RenderItems(self.clone())])
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum MySecondDomainProcessingError {
    #[error("The todo at index {0} does not exist!")]
    ItemDoesNotExist(usize),
    #[error("This is a second Error!")]
    SecondError,
}

impl CqrsModel for MySecondDomainModel {}
