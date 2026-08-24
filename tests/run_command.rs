use std::{
    fs,
    path::{Path, PathBuf},
    process::{self, Command},
    time::{SystemTime, UNIX_EPOCH},
};

fn create_project(test_name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let project =
        std::env::temp_dir().join(format!("nodmodcomp-{test_name}-{}-{unique}", process::id()));

    fs::create_dir_all(project.join("node_modules/example")).unwrap();
    fs::write(project.join("package.json"), b"{}").unwrap();
    fs::write(project.join("node_modules/example/index.js"), b"module").unwrap();

    project
}

fn run_in(project: &Path, script: &str) -> process::ExitStatus {
    Command::new(env!("CARGO_BIN_EXE_nodmodcomp"))
        .current_dir(project)
        .args(["run", "--", "sh", "-c", script])
        .status()
        .unwrap()
}

fn run_with_output(project: &Path, script: &str) -> process::Output {
    Command::new(env!("CARGO_BIN_EXE_nodmodcomp"))
        .current_dir(project)
        .args(["run", "--", "sh", "-c", script])
        .output()
        .unwrap()
}

fn pack_project(project: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_nodmodcomp"))
        .current_dir(project)
        .arg("pack")
        .status()
        .unwrap();

    assert!(status.success());
    assert_project_is_packed(project);
}

fn assert_project_is_packed(project: &Path) {
    assert!(!project.join("node_modules").exists());
    assert!(project.join("node_modules.pack").is_file());
    assert!(project.join("node_modules.pack.meta.json").is_file());
}

#[test]
fn repacks_after_successful_command() {
    let project = create_project("run-success");
    pack_project(&project);

    let status = run_in(&project, "test -f node_modules/example/index.js");

    assert!(status.success());
    assert_project_is_packed(&project);
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn repacks_and_preserves_a_failed_commands_exit_code() {
    let project = create_project("run-failure");
    pack_project(&project);

    let status = run_in(&project, "exit 7");

    assert_eq!(status.code(), Some(7));
    assert_project_is_packed(&project);
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn leaves_already_unpacked_node_modules_unchanged() {
    let project = create_project("run-already-unpacked");

    let output = run_with_output(&project, "test -f node_modules/example/index.js");

    assert!(output.status.success());
    assert!(project.join("node_modules/example/index.js").is_file());
    assert!(!project.join("node_modules.pack").exists());
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("node_modules was already unpacked; skipping repack.")
    );
    fs::remove_dir_all(project).unwrap();
}
