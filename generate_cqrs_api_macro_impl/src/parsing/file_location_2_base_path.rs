use crate::generate_api_macro_impl::BasePath;

pub(crate) fn file_location_2_base_path(file_path: &str) -> BasePath {
    let mut path_split = file_path
        .split('/')
        .skip_while(|element| *element != "src" && *element != "tests");
    path_split
        .next()
        .expect("file path needs to contain 'src/' or 'tests/'");
    let dirty_result = format!("crate::{}", path_split.collect::<Vec<&str>>().join("::"));
    let path = match &dirty_result[dirty_result.rfind("::").unwrap()..] {
        "::mod.rs" => dirty_result[..dirty_result.len() - 8].to_string(),
        file if file.ends_with(".rs") => dirty_result[..dirty_result.len() - 3].to_string(),
        file => {
            panic!(
                "File location doesn't end with a '.rs' file: '{}'",
                &file[2..]
            );
        }
    };
    BasePath(path)
}

#[cfg(test)]
mod tests {
    use crate::generate_api_macro_impl::BasePath;

    use super::file_location_2_base_path;

    #[test]
    #[should_panic(expected = "file path needs to contain 'src/'")]
    fn test_file_file_location_2_base_path_no_src() {
        file_location_2_base_path("main.rs");
    }

    #[test]
    #[should_panic(expected = "File location doesn't end with a '.rs' file: 'main'")]
    fn test_file_file_location_2_base_path_main_no_rs() {
        file_location_2_base_path("src/module/main");
    }
    #[test]
    #[should_panic(expected = "File location doesn't end with a '.rs' file: 'mod'")]
    fn test_file_file_location_2_base_path_no_rs() {
        file_location_2_base_path("src/module/mod");
    }
    #[test]
    fn test_file_file_location_2_base_path_zero_levels() {
        assert_eq!(
            BasePath("crate::main".to_string()),
            file_location_2_base_path("src/main.rs")
        );
    }
    #[test]
    fn test_file_file_location_2_base_path_one_level() {
        assert_eq!(
            BasePath("crate::domain::model".to_string()),
            file_location_2_base_path("src/domain/model.rs")
        );
    }
    #[test]
    fn test_file_file_location_2_base_path_two_levels() {
        assert_eq!(
            BasePath("crate::domain::model::item".to_string()),
            file_location_2_base_path("src/domain/model/item.rs")
        );
    }
    #[test]
    fn test_file_file_location_2_base_path_multiple_levels() {
        assert_eq!(
            BasePath("crate::domain::model::items::entity".to_string()),
            file_location_2_base_path("src/domain/model/items/entity.rs")
        );
    }
    #[test]
    fn test_file_file_location_2_base_path_one_level_with_mod() {
        assert_eq!(
            BasePath("crate::domain::model".to_string()),
            file_location_2_base_path("src/domain/model/mod.rs")
        );
    }
    #[test]
    fn test_file_file_location_2_base_path_two_levels_with_mod() {
        assert_eq!(
            BasePath("crate::domain::model::item".to_string()),
            file_location_2_base_path("src/domain/model/item/mod.rs")
        );
    }
    #[test]
    fn test_file_file_location_2_base_path_multiple_levels_with_mod() {
        assert_eq!(
            BasePath("crate::domain::model::items::entity".to_string()),
            file_location_2_base_path("src/domain/model/items/entity/mod.rs")
        );
    }
}
