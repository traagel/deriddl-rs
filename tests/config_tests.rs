mod common;
use common::deri_ddl_cmd;
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

#[test]
#[serial]
fn test_config_generation() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    deri_ddl_cmd()
        .arg("config")
        .arg("--output")
        .arg(&config_path)
        .current_dir(&temp_dir)
        .assert()
        .success();

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[database]"));
}

#[test]
#[serial]
fn test_config_generation_with_env() {
    let temp_dir = tempdir().unwrap();

    deri_ddl_cmd()
        .arg("config")
        .arg("--env")
        .arg("test")
        .current_dir(&temp_dir)
        .assert()
        .success();

    assert!(temp_dir.path().join("config.toml").exists());
    assert!(temp_dir.path().join("config/test.toml").exists());
}
