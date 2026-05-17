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
fn list_done_shows_completed() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Done task"]).assert().success();
    task(&scope).args(["complete", "1"]).assert().success();
    task(&scope)
        .args(["list", "done"])
        .assert()
        .success()
        .stdout(contains("Done task"));
}

#[test]
fn list_all_shows_both_active_and_completed() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Active"]).assert().success();
    task(&scope).args(["add", "Completed"]).assert().success();
    task(&scope).args(["complete", "2"]).assert().success();
    task(&scope)
        .args(["list", "--all"])
        .assert()
        .success()
        .stdout(contains("Active"))
        .stdout(contains("Completed"));
}

use predicates::prelude::*;
