use generate_cqrs_api_macros::generate_api;

// include!("good_source_file/mod.rs");
#[test]
fn generate_api_test() {
    generate_api!("tests/src/good_source_file.rs");
    // generate_api!("src/good_source_file.rs");
}
