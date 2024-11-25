pub use crate::AppConfig;

#[derive(Debug, Default)]
pub(crate) struct AppConfigImpl {
    app_state_file_path: std::path::PathBuf,
}
impl AppConfigImpl {}

impl AppConfig for AppConfigImpl {
    fn new(path: Option<String>) -> Self {
        Self {
            app_state_file_path: std::env::current_dir()
                .unwrap()
                .join(path.unwrap_or("app-state".to_string())),
        }
    }
    // app state storage location
    fn get_app_state_file_path(&self) -> &std::path::PathBuf {
        &self.app_state_file_path
    }
}
