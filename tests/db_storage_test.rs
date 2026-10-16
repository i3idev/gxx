//! Regression tests for local/global database and vault behavior.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn gxx() -> Command {
    Command::cargo_bin("gxx").unwrap()
}

#[test]
fn vault_flag_beats_env_vault() {
    let root = tempdir().unwrap();
    let env_vault = root.path().join("envvault");
    let cli_vault = root.path().join("clivault");

    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &env_vault);
    cmd.arg("--vault").arg(&cli_vault).arg("init");
    cmd.assert().success();

    assert!(
        cli_vault.join(".gxx").exists(),
        "CLI vault should be initialized"
    );
    assert!(
        !env_vault.join(".gxx").exists(),
        "env vault must not be touched"
    );
}

#[test]
fn local_and_global_same_name_are_independent() {
    let root = tempdir().unwrap();
    let global_home = root.path().join("global");
    let local_vault = root.path().join("project");

    // Global database.
    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .arg("init")
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .args(["db", "new", "demo"])
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .args(["db", "use", "demo"])
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .args(["add", "globalword", "ترجمه", "-p", "n"])
        .assert()
        .success();

    // Local database with the same name.
    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .arg("init")
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .args(["db", "new", "demo"])
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .args(["db", "use", "demo"])
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .args(["add", "localword", "meaning", "-p", "n"])
        .assert()
        .success();

    // Global scope does not see the local word.
    let out = gxx()
        .env("GXX_HOME", &global_home)
        .arg("list")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("globalword"), "global list: {stdout}");
    assert!(
        !stdout.contains("localword"),
        "global list leaked local: {stdout}"
    );

    // Local scope does not see the global word.
    let out = gxx()
        .env("GXX_VAULT", &local_vault)
        .arg("list")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("localword"), "local list: {stdout}");
    assert!(
        !stdout.contains("globalword"),
        "local list leaked global: {stdout}"
    );
}

#[test]
fn missing_local_database_does_not_fall_back_to_global() {
    let root = tempdir().unwrap();
    let global_home = root.path().join("global");
    let local_vault = root.path().join("project");

    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .arg("init")
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .args(["db", "new", "demo"])
        .assert()
        .success();

    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .arg("init")
        .assert()
        .success();

    // The local scope has no active database. It must not use the global one.
    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .args(["show", "anything"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("no active db"));
}

#[test]
fn explicit_db_override_uses_selected_scope_only() {
    let root = tempdir().unwrap();
    let global_home = root.path().join("global");
    let local_vault = root.path().join("project");

    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .arg("init")
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .args(["db", "new", "vaultdb"])
        .assert()
        .success();

    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .arg("init")
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .args(["db", "new", "localdb"])
        .assert()
        .success();

    // Selecting the global DB name from the local scope must not silently
    // open or create a file in the local vault.
    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .args(["--db", "vaultdb", "list"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("no database 'vaultdb'"));

    assert!(!local_vault.join(".gxx/db/vaultdb.db").exists());
}

#[test]
fn invalid_vault_path_reports_not_initialised() {
    let missing = tempdir().unwrap().path().join("does-not-exist");

    let mut cmd = gxx();
    cmd.arg("--vault").arg(&missing).arg("db").arg("list");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("not initialised"));
}

#[test]
fn active_selections_do_not_overwrite_each_other() {
    let root = tempdir().unwrap();
    let global_home = root.path().join("global");
    let local_vault = root.path().join("project");

    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .arg("init")
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .args(["db", "new", "globaldb"])
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .args(["db", "use", "globaldb"])
        .assert()
        .success();

    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .arg("init")
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .args(["db", "new", "localdb"])
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .args(["db", "use", "localdb"])
        .assert()
        .success();

    // Global active stays globaldb.
    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home).args(["db", "info"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("active"),
        "global info missing active line: {stdout}"
    );
    // info output includes database name.
    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .args(["db", "info", "globaldb"])
        .assert()
        .success();

    // Local active remains localdb.
    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .args(["db", "info", "localdb"])
        .assert()
        .success();
}

#[test]
fn deleting_in_one_scope_does_not_delete_other_scope() {
    let root = tempdir().unwrap();
    let global_home = root.path().join("global");
    let local_vault = root.path().join("project");

    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .arg("init")
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_HOME", &global_home)
        .args(["db", "new", "demo"])
        .assert()
        .success();

    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .arg("init")
        .assert()
        .success();
    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .args(["db", "new", "demo"])
        .assert()
        .success();

    // Delete local demo.
    let mut cmd = gxx();
    cmd.env("GXX_VAULT", &local_vault)
        .args(["db", "del", "demo", "--yes"])
        .assert()
        .success();
    assert!(!local_vault.join(".gxx/db/demo.db").exists());

    // Global demo survives.
    assert!(global_home.join("db/demo.db").exists());
}
