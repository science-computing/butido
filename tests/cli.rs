use std::process::Command;  // Run programs
use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions

#[test]
fn test_cli_build() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("butido")?;

    cmd.current_dir(tmpdir.as_ref())
        .arg("build")
        .arg("example")
        .arg("-I")
        .arg("foo:bar")
        .arg("example")
        .arg("1.0.0");

    // butido cannot do anything if there are no packages. So this should fail.
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory"));

    Ok(())
}
