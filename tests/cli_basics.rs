mod common;
use common::deri_ddl_cmd;
use predicates::prelude::*;

#[test]
fn test_help_command() {
    deri_ddl_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Rust-based ODBC schema migration runner",
        ));
}

#[test]
fn test_version_command() {
    deri_ddl_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("deriddl")); // lowercase
}

#[test]
fn test_invalid_subcommand() {
    deri_ddl_cmd()
        .arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_missing_subcommand() {
    deri_ddl_cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage: deriDDL")); // updated match
}
