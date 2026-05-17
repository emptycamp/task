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
fn revert_nothing_to_revert_fails() {
    let scope = StoreScope::new();
    task(&scope).args(["revert", "-y"]).assert().failure();
}

#[test]
fn revert_add_removes_task() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Temporary"]).assert().success();
    task(&scope).args(["revert", "-y"]).assert().success();
    task(&scope)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("No tasks"));
}

#[test]
fn revert_delete_restores_task() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Important"]).assert().success();
    task(&scope).args(["delete", "1"]).assert().success();
    task(&scope).args(["revert", "-y"]).assert().success();
    task(&scope)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("Important"));
}

#[test]
fn revert_complete_restores_to_active() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Not done yet"]).assert().success();
    task(&scope).args(["complete", "1"]).assert().success();
    task(&scope).args(["revert", "-y"]).assert().success();
    task(&scope)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("Not done yet"));
}

#[test]
fn revert_stacks_multiple_operations() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Task 1"]).assert().success();
    task(&scope).args(["add", "Task 2"]).assert().success();
    task(&scope).args(["revert", "-y"]).assert().success();
    task(&scope)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("Task 1"))
        .stdout(predicates::str::contains("Task 2").not());
}

use predicates::prelude::*;
