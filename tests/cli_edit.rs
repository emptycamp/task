mod support;

use assert_cmd::Command;
use predicates::str::contains;
use support::StoreScope;

fn task(scope: &StoreScope) -> Command {
    let mut cmd = Command::cargo_bin("task").unwrap();
    cmd.env("TASK_DATA_DIR", &scope.path);
    cmd
}

fn fake_editor_path() -> std::path::PathBuf {
    assert_cmd::cargo::cargo_bin("fake-editor")
}

#[test]
fn edit_priority_via_args() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Read book"]).assert().success();
    task(&scope)
        .args(["edit", "1", "p:a"])
        .assert()
        .success();
    task(&scope)
        .args(["info", "1"])
        .assert()
        .success()
        .stdout(contains("A"));
}

#[test]
fn edit_text_via_args() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Old text"]).assert().success();
    task(&scope)
        .args(["edit", "1", "New text"])
        .assert()
        .success();
    task(&scope)
        .args(["info", "1"])
        .assert()
        .success()
        .stdout(contains("New text"));
}

#[test]
fn edit_alias_update_works() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Task"]).assert().success();
    task(&scope)
        .args(["update", "1", "p:c"])
        .assert()
        .success();
}

#[test]
fn edit_nonexistent_task_fails() {
    let scope = StoreScope::new();
    task(&scope)
        .args(["edit", "99", "p:a"])
        .assert()
        .failure();
}

#[test]
fn edit_with_fake_editor_updates_task() {
    let scope = StoreScope::new();
    task(&scope).args(["add", "Buy milk"]).assert().success();

    // Get the YAML for task 1, then inject new content via fake-editor
    let new_content = r#"text: Buy oat milk
priority: A
due: 2026-06-01T09:00:00Z
est: 15m
"#;

    task(&scope)
        .args(["edit", "1"])
        .env("EDITOR", fake_editor_path())
        .env("FAKE_EDITOR_CONTENT", new_content)
        .assert()
        .success();

    task(&scope)
        .args(["info", "1"])
        .assert()
        .success()
        .stdout(contains("Buy oat milk"));
}
