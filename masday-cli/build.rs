use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Get the workspace root (parent of masday-cli)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&manifest_dir).parent().unwrap();
    let agents_src = workspace_root.join(".claude/agents");
    let skills_src = workspace_root.join(".claude/skills");
    let hooks_src = workspace_root.join(".claude/hooks");
    let global_hooks_src = workspace_root.join("scripts/global-hooks");
    let git_hooks_src = workspace_root.join("scripts/git-hooks");
    let scripts_src = workspace_root.join("scripts");

    println!("cargo:rerun-if-changed={}", agents_src.display());
    println!("cargo:rerun-if-changed={}", skills_src.display());
    println!("cargo:rerun-if-changed={}", global_hooks_src.display());
    println!("cargo:rerun-if-changed={}", hooks_src.display());
    println!("cargo:rerun-if-changed={}", git_hooks_src.display());
    println!(
        "cargo:rerun-if-changed={}",
        scripts_src.join("registry-sync.mjs").display()
    );

    let out_dir = env::var("OUT_DIR").unwrap();
    let templates_dir = Path::new(&out_dir).join("templates");

    // Create templates directory structure
    let agents_dir = templates_dir.join("agents");
    let skills_dir = templates_dir.join("skills");
    let global_hooks_dir = templates_dir.join("global-hooks");
    let git_hooks_dir = templates_dir.join("git-hooks");
    let project_hooks_dir = templates_dir.join("project-hooks");
    let scripts_dir = templates_dir.join("scripts");

    fs::create_dir_all(&agents_dir).unwrap();
    fs::create_dir_all(&skills_dir).unwrap();
    fs::create_dir_all(&global_hooks_dir).unwrap();
    fs::create_dir_all(&git_hooks_dir).unwrap();
    fs::create_dir_all(&project_hooks_dir).unwrap();
    fs::create_dir_all(&scripts_dir).unwrap();

    // Copy agent .md files
    if let Ok(entries) = fs::read_dir(&agents_src) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(filename) = path.file_name() {
                    let dest = agents_dir.join(filename);
                    fs::copy(&path, &dest).unwrap();
                }
            }
        }
    }

    // Copy skill directories (only masday-*)
    if let Ok(entries) = fs::read_dir(&skills_src) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if dir_name.starts_with("masday-") {
                        let dest_skill_dir = skills_dir.join(dir_name);
                        fs::create_dir_all(&dest_skill_dir).unwrap();
                        copy_dir_recursive(&path, &dest_skill_dir).unwrap();
                    }
                }
            }
        }
    }

    // Copy global hooks
    if let Ok(entries) = fs::read_dir(&global_hooks_src) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name() {
                    let dest = global_hooks_dir.join(filename);
                    fs::copy(&path, &dest).unwrap();
                }
            }
        }
    }

    // Copy git hooks (pre-commit, pre-push, etc.)
    if let Ok(entries) = fs::read_dir(&git_hooks_src) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name() {
                    let dest = git_hooks_dir.join(filename);
                    fs::copy(&path, &dest).unwrap();
                }
            }
        }
    }

    // Copy project hooks (masday-*.cjs, masday-*.js, run.sh, skill-step-guard.cjs)
    if let Ok(entries) = fs::read_dir(&hooks_src) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    let should_copy = filename.starts_with("masday-")
                        && (filename.ends_with(".cjs") || filename.ends_with(".js"))
                        || filename == "run.sh"
                        || filename == "skill-step-guard.cjs";

                    if should_copy {
                        let dest = project_hooks_dir.join(filename);
                        fs::copy(&path, &dest).unwrap();
                    }
                }
            }
        }
    }

    // Copy utility scripts (registry-sync.mjs, etc.)
    for script_name in &["registry-sync.mjs"] {
        let src = scripts_src.join(script_name);
        if src.exists() {
            let dest = scripts_dir.join(script_name);
            fs::copy(&src, &dest).unwrap();
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if dst.is_dir() {
        fs::remove_dir_all(dst)?;
    }
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        // Skip symlinks — prevent arbitrary file read outside source tree
        if src_path.is_symlink() {
            continue;
        }

        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
