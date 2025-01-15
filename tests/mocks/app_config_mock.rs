#[derive(Debug, Default)]
pub struct AppConfigImpl {
    app_state_file_path: String,
}
impl AppConfigImpl {}

impl AppConfig for AppConfigImpl {
    fn new(path: Option<String>) -> Self {
        Self {
            app_state_file_path: std::env::current_dir()
                .unwrap()
                .join(path.unwrap_or("app-state".to_string()))
                .to_string_lossy()
                .to_string(),
        }
    }
    // app state storage location
    fn borrow_app_state_url(&self) -> &str {
        &self.app_state_file_path
    }
}
