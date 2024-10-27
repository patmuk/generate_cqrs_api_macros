use generate_cqrs_api_macros::generate_api;

mod good_source_file;

include!("good_source_file/mod.rs");
#[test]
fn generate_api_test() {
    generate_api!("tests/good_source_file/mod.rs");
}
