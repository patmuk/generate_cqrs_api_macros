use std::fmt::Debug;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct AppStatePersisterMock {
    pub(crate) _path: PathBuf,
}

// handle errors as suggested by https://kazlauskas.me/entries/errors
#[derive(thiserror::Error, Debug)]
pub enum AppStatePersisterErrorMock {
    #[error("No File found in: {0}")]
    FileNotFound(String),
}

impl AppStatePersistError for AppStatePersisterErrorMock {
    fn to_processing_error(&self) -> ProcessingError {
        match self {
            AppStatePersisterErrorMock::FileNotFound(path) => ProcessingError::NotPersisted {
                error: self.to_string(),
                url: path.to_owned(),
            },
        }
    }
}

impl From<(io::Error, String)> for AppStatePersisterErrorMock {
    fn from((_err, path): (io::Error, String)) -> Self {
        AppStatePersisterErrorMock::FileNotFound(path)
    }
}

/// Persists the application state to storage (a file).
/// Ensures that the `AppState` is stored in a durable way, regardless of the underlying mechanism.
impl AppStatePersister for AppStatePersisterMock {
    type Error = AppStatePersisterErrorMock;
    fn new<AC: AppConfig>(app_config: &AC) -> Result<Self, Self::Error> {
        // create the directories, but no need to write the file, as there is only the default content
        // remove the last part, as this is the file
        let path = PathBuf::from(app_config.borrow_app_state_url());
        Ok(AppStatePersisterMock {
            _path: path.to_owned(),
        })
    }

    fn persist_app_state<AS: AppState + Serialize + std::fmt::Debug>(
        &self,
        _app_state: &AS,
    ) -> Result<(), Self::Error> {
        unimplemented!();
    }

    // get the last persisted app state from a file, if any exists, otherwise creates a new app state
    // this function is only called once, in the initialization/app state constructor
    fn load_app_state<AC: AppConfig, AS: AppState + for<'a> Deserialize<'a>>(
        &self,
    ) -> Result<AS, Self::Error> {
        unimplemented!()
    }
}
