use std::path::PathBuf;

#[cfg(test)]
use log::info;
use simple_logger::SimpleLogger;
#[test]
fn test_compile_macro() {
    SimpleLogger::new().init().unwrap();
    info!("test compile generate_api macro");
    let cwd = std::env::current_dir().expect("can't get cwd!");
    std::env::set_var("$DIR", cwd);
    let env_key = "$DIR";
    let mut dir = std::env::var(env_key).unwrap_or_else(|_| panic!("can't read env {}", env_key));
    info!("{} before calling trybuild: {}", env_key, dir);
    let try_build = trybuild::TestCases::new();
    dir = std::env::var(env_key)
        .unwrap_or_else(|_| panic!("can't read after trybuild env {}", env_key));
    info!("{} after calling trybuild: {}", env_key, dir);
    try_build.pass(PathBuf::from(format!("{}/tests/ui_tests.rs", dir)));
    // try_build.pass("tests/ui_tests.rs");
    dir = std::env::var(env_key)
        .unwrap_or_else(|_| panic!("can't read after trybuild.pass env {}", env_key));
    info!("{} after calling trybuild.pass: {}", env_key, dir)
}
