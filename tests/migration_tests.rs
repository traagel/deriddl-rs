mod common;
use common::{deri_ddl_cmd, setup_test_migrations};
use predicates::str::contains;

#[test]
fn test_status_command_no_connection() {
    let temp_dir = setup_test_migrations();

    deri_ddl_cmd()
        .arg("status")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .stdout(contains("No connection string provided"));
}

#[test]
fn test_apply_command_no_connection() {
    let temp_dir = setup_test_migrations();

    deri_ddl_cmd()
        .arg("apply")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .stdout(contains("No connection string provided"));
}
