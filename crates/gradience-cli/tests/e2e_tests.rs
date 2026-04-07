use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;

fn cmd_with_temp() -> (Command, tempfile::TempDir) {
    let temp = tempfile::tempdir().unwrap();

    // Auto-login so agent commands have a session
    let mut login = Command::cargo_bin("gradience").unwrap();
    login.env("GRADIENCE_DATA_DIR", temp.path());
    login.args(["auth", "login"]);
    login.write_stdin("demo-pass-12345\n");
    login.assert().success();

    let mut cmd = Command::cargo_bin("gradience").unwrap();
    cmd.env("GRADIENCE_DATA_DIR", temp.path());
    (cmd, temp)
}

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("gradience").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Gradience Wallet"));
}

#[test]
fn test_agent_create_success() {
    let (mut cmd, _temp) = cmd_with_temp();
    cmd.args(["agent", "create", "--name", "e2e-demo"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created wallet"));
}

#[test]
fn test_agent_create_empty_name_fails() {
    let (mut cmd, _temp) = cmd_with_temp();
    cmd.args(["agent", "create", "--name", ""]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("empty"));
}

#[test]
fn test_agent_balance_success() {
    let (mut cmd, _temp) = cmd_with_temp();
    cmd.args(["agent", "create", "--name", "balance-test"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let out_str = String::from_utf8_lossy(&out);
    let wallet_id = out_str.lines().find(|l| l.contains("id:")).map(|l| {
        l.split("id:").nth(1).unwrap().trim().trim_matches(')').split_whitespace().next().unwrap()
    }).unwrap_or("wallet-123");

    let mut cmd2 = Command::cargo_bin("gradience").unwrap();
    cmd2.env("GRADIENCE_DATA_DIR", _temp.path());
    cmd2.args(["agent", "balance", wallet_id, "--chain", "base"]);
    cmd2.assert()
        .success()
        .stdout(predicate::str::contains("0x"));
}

#[test]
fn test_policy_set_success() {
    let (mut cmd, _temp) = cmd_with_temp();
    let mut temp = tempfile::NamedTempFile::new().unwrap();
    write!(temp, r#"{{"rules":[]}}"#).unwrap();
    let path = temp.path().to_str().unwrap();

    cmd.args(["policy", "set", "wallet-123", "--file", path]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Wallet not found"));
}

#[test]
fn test_policy_set_invalid_json_fails() {
    let (mut cmd, _temp) = cmd_with_temp();
    let mut temp = tempfile::NamedTempFile::new().unwrap();
    write!(temp, "not json").unwrap();
    let path = temp.path().to_str().unwrap();

    cmd.args(["policy", "set", "wallet-123", "--file", path]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid policy JSON"));
}
