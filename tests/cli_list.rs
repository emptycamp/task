mod support;

use assert_cmd::Command;
use predicates::str::contains;
use support::StoreScope;

fn task(scope: &StoreScope) -> Command {
    let mut cmd = Command::cargo_bin("task").unwrap();
    cmd.env("TASK_DATA_DIR", &scope.path);
    cmd
}

#[test]
fn list_empty_store_shows_no_tasks() {
    let scope = StoreScope::new();
    task(&scope)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("No tasks"));
}

#[test]
fn list_alias_ls_works() {
    let scope = StoreScope::new();
    task(&scope)
        .args(["ls"])
        .assert()
        .success();
}

#[test]
fn list_shows_active_tasks() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Active task"]).assert().success();
    task(&scope)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("Active task"));
}

#[test]
fn list_does_not_show_completed_tasks() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Done task"]).assert().success();
    task(&scope).args(["complete", "1"]).assert().success();
    task(&scope)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Done task").not());
}

#[test]
fn list_active_flag_shows_only_active() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Active"]).assert().success();
    task(&scope).args(["add", "Done"]).assert().success();
    task(&scope).args(["complete", "2"]).assert().success();
    task(&scope)
        .args(["list", "-a"])
        .assert()
        .success()
        .stdout(contains("Active"))
        .stdout(predicates::str::contains("Done").not());
}

#[test]
fn list_completed_flag_shows_only_completed() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Active"]).assert().success();
    task(&scope).args(["add", "Done task"]).assert().success();
    task(&scope).args(["complete", "2"]).assert().success();
    task(&scope)
        .args(["list", "--completed"])
        .assert()
        .success()
        .stdout(contains("Done task"))
        .stdout(predicates::str::contains("Active").not());
}

#[test]
fn list_completed_alias_done_works() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Done task"]).assert().success();
    task(&scope).args(["complete", "1"]).assert().success();
    task(&scope)
        .args(["list", "--done"])
        .assert()
        .success()
        .stdout(contains("Done task"));
}

#[test]
fn list_completed_alias_finished_works() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Done task"]).assert().success();
    task(&scope).args(["complete", "1"]).assert().success();
    task(&scope)
        .args(["list", "--finished"])
        .assert()
        .success()
        .stdout(contains("Done task"));
}

#[test]
fn list_deleted_flag_shows_only_deleted() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Keep me"]).assert().success();
    task(&scope).args(["add", "Trashed"]).assert().success();
    task(&scope).args(["delete", "2"]).assert().success();
    task(&scope)
        .args(["list", "--deleted"])
        .assert()
        .success()
        .stdout(contains("Trashed"))
        .stdout(predicates::str::contains("Keep me").not());
}

#[test]
fn list_deleted_alias_trash_works() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Trashed"]).assert().success();
    task(&scope).args(["delete", "1"]).assert().success();
    task(&scope)
        .args(["list", "--trash"])
        .assert()
        .success()
        .stdout(contains("Trashed"));
}

#[test]
fn list_all_shows_active_completed_and_deleted() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Active task"]).assert().success();
    task(&scope).args(["add", "Done task"]).assert().success();
    task(&scope).args(["add", "Trashed task"]).assert().success();
    task(&scope).args(["complete", "2"]).assert().success();
    task(&scope).args(["delete", "3"]).assert().success();
    task(&scope)
        .args(["list", "--all"])
        .assert()
        .success()
        .stdout(contains("Active task"))
        .stdout(contains("Done task"))
        .stdout(contains("Trashed task"));
}

#[test]
fn list_help_includes_examples_for_all_filters() {
    let scope = StoreScope::new();
    task(&scope)
        .args(["list", "--help"])
        .assert()
        .success()
        .stdout(contains("--completed"))
        .stdout(contains("--deleted"))
        .stdout(contains("--active"))
        .stdout(contains("--all"))
        .stdout(contains("Examples"));
}

#[test]
fn list_output_includes_header() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Active task"]).assert().success();
    task(&scope)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("ID"))
        .stdout(contains("Description"))
        .stdout(contains("Due"))
        .stdout(contains("Est"));
}

use predicates::prelude::*;
