use crate::generate_api_macro_impl::BasePath;

pub(crate) fn file_location_2_base_paths(file_paths: Vec<String>) -> Vec<BasePath> {
    file_paths
        .iter()
        .map(|file_path| {
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
        })
        .collect::<Vec<BasePath>>()
}

#[cfg(test)]
mod tests {
    use crate::generate_api_macro_impl::BasePath;

    use super::file_location_2_base_paths;

    #[test]
    #[should_panic(expected = "file path needs to contain 'src/'")]
    fn test_file_file_location_2_base_paths_no_src() {
        file_location_2_base_paths(vec!["main.rs".to_string()]);
    }

    #[test]
    #[should_panic(expected = "File location doesn't end with a '.rs' file: 'main'")]
    fn test_file_file_location_2_base_paths_main_no_rs() {
        file_location_2_base_paths(vec!["src/module/main".to_string()]);
    }
    #[test]
    #[should_panic(expected = "File location doesn't end with a '.rs' file: 'mod'")]
    fn test_file_file_location_2_base_paths_no_rs() {
        file_location_2_base_paths(vec!["src/module/mod".to_string()]);
    }
    #[test]
    fn test_file_file_location_2_base_paths_zero_levels() {
        assert_eq!(
            vec![BasePath("crate::main".to_string())],
            file_location_2_base_paths(vec!["src/main.rs".to_string()])
        );
    }
    #[test]
    fn test_file_file_location_2_base_paths_one_level() {
        assert_eq!(
            vec![BasePath("crate::domain::model".to_string())],
            file_location_2_base_paths(vec!["src/domain/model.rs".to_string()])
        );
    }
    #[test]
    fn test_file_file_location_2_base_paths_two_levels() {
        assert_eq!(
            vec![BasePath("crate::domain::model::item".to_string())],
            file_location_2_base_paths(vec!["src/domain/model/item.rs".to_string()])
        );
    }
    #[test]
    fn test_file_file_location_2_base_paths_multiple_levels() {
        assert_eq!(
            vec![BasePath("crate::domain::model::items::entity".to_string())],
            file_location_2_base_paths(vec!["src/domain/model/items/entity.rs".to_string()])
        );
    }
    #[test]
    fn test_file_file_location_2_base_paths_multiple_models() {
        let input = vec![
            "src/domain/model/items/entity.rs".to_string(),
            "src/domain/model/items/second_entity.rs".to_string(),
        ];
        assert_eq!(
            vec![
                BasePath("crate::domain::model::items::entity".to_string()),
                BasePath("crate::domain::model::items::second_entity".to_string())
            ],
            file_location_2_base_paths(input)
        );
    }
    #[test]
    fn test_file_file_location_2_base_paths_multiple_models_different_paths() {
        let input = vec![
            "src/domain/model/items/entity.rs".to_string(),
            "src/domain/model/secondary_items/second_entity.rs".to_string(),
        ];
        assert_eq!(
            vec![
                BasePath("crate::domain::model::items::entity".to_string()),
                BasePath("crate::domain::model::secondary_items::second_entity".to_string())
            ],
            file_location_2_base_paths(input)
        );
    }
    #[test]
    fn test_file_file_location_2_base_paths_one_level_with_mod() {
        assert_eq!(
            vec![BasePath("crate::domain::model".to_string())],
            file_location_2_base_paths(vec!["src/domain/model/mod.rs".to_string()])
        );
    }
    #[test]
    fn test_file_file_location_2_base_paths_two_levels_with_mod() {
        assert_eq!(
            vec![BasePath("crate::domain::model::item".to_string())],
            file_location_2_base_paths(vec!["src/domain/model/item/mod.rs".to_string()])
        );
    }
    #[test]
    fn test_file_file_location_2_base_paths_multiple_levels_with_mod() {
        assert_eq!(
            vec![BasePath("crate::domain::model::items::entity".to_string())],
            file_location_2_base_paths(vec!["src/domain/model/items/entity/mod.rs".to_string()])
        );
    }
    #[test]
    fn test_file_file_location_2_base_paths_multiple_rust_files_with_mod() {
        let input = vec![
            "src/domain/model/items/entity/mod.rs".to_string(),
            "src/domain/model/items/second_entity/mod.rs".to_string(),
        ];
        assert_eq!(
            vec![
                BasePath("crate::domain::model::items::entity".to_string()),
                BasePath("crate::domain::model::items::second_entity".to_string())
            ],
            file_location_2_base_paths(input)
        );
    }
    #[test]
    fn test_file_file_location_2_base_paths_multiple_rust_files_different_path_with_mod() {
        let input = vec![
            "src/domain/model/items/entity/mod.rs".to_string(),
            "src/domain/model/secondary_items/second_entity/mod.rs".to_string(),
        ];
        assert_eq!(
            vec![
                BasePath("crate::domain::model::items::entity".to_string()),
                BasePath("crate::domain::model::secondary_items::second_entity".to_string())
            ],
            file_location_2_base_paths(input)
        );
    }
}
