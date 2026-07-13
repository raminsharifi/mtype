use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn git_output(manifest_dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(manifest_dir)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn emit_git_rerun_paths(manifest_dir: &Path) {
    let Some(git_dir) = git_output(manifest_dir, &["rev-parse", "--git-dir"]) else {
        return;
    };
    let git_dir = PathBuf::from(git_dir);
    let git_dir = if git_dir.is_absolute() {
        git_dir
    } else {
        manifest_dir.join(git_dir)
    };
    let head = git_dir.join("HEAD");
    println!("cargo:rerun-if-changed={}", head.display());
    println!("cargo:rerun-if-changed={}", git_dir.join("index").display());
    println!(
        "cargo:rerun-if-changed={}",
        git_dir.join("packed-refs").display()
    );

    if let Ok(contents) = fs::read_to_string(&head) {
        if let Some(reference) = contents.trim().strip_prefix("ref: ") {
            println!(
                "cargo:rerun-if-changed={}",
                git_dir.join(reference).display()
            );
        }
    }
}

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    println!("cargo:rerun-if-env-changed=MTYPE_GIT_HASH");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=Cargo.toml");
    emit_git_rerun_paths(&manifest_dir);

    let revision = env::var("MTYPE_GIT_HASH").ok().unwrap_or_else(|| {
        let hash = git_output(&manifest_dir, &["rev-parse", "--short=8", "HEAD"])
            .unwrap_or_else(|| "unknown".to_string());
        let dirty = git_output(
            &manifest_dir,
            &["status", "--porcelain", "--untracked-files=normal"],
        )
        .is_some_and(|status| !status.is_empty());
        if dirty && hash != "unknown" {
            format!("{hash}-dirty")
        } else {
            hash
        }
    });

    println!("cargo:rustc-env=MTYPE_BUILD_GIT_HASH={revision}");
}
