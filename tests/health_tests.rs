mod common;
use common::{deri_ddl_cmd, setup_test_migrations};

#[test]
fn test_health_command_default() {
    let temp_dir = setup_test_migrations();

    deri_ddl_cmd()
        .arg("health")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .current_dir(&temp_dir)
        .assert()
        .success();
}

#[test]
fn test_health_command_custom_dialect() {
    let temp_dir = setup_test_migrations();

    deri_ddl_cmd()
        .arg("health")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .arg("--dialect")
        .arg("mysql")
        .current_dir(&temp_dir)
        .assert()
        .success();
}

#[test]
fn test_health_command_nonexistent_path() {
    deri_ddl_cmd()
        .arg("health")
        .arg("--path")
        .arg("/nonexistent/path")
        .assert()
        .failure();
}
