use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub const LAUNCHER_FILE_NAME: &str = "ccp-copy.desktop";
const LAUNCHER_MARKER: &str = "X-CCP-Launcher=true";

pub fn create_launcher(root: &Path, force: bool) -> Result<PathBuf> {
    create_launcher_for_environment(
        root,
        force,
        cfg!(target_os = "linux"),
        cfg!(feature = "clipboard"),
    )
}

fn create_launcher_for_environment(
    root: &Path,
    force: bool,
    linux_platform: bool,
    clipboard_enabled: bool,
) -> Result<PathBuf> {
    if !clipboard_enabled {
        bail!(
            "Launcher creation requires clipboard support. Reinstall with: cargo install ccp_tree --features clipboard"
        );
    }
    if !linux_platform {
        bail!("Launcher creation is currently supported only on Linux desktops.");
    }

    if !root.exists() {
        bail!(
            "Launcher target directory does not exist: {}",
            root.display()
        );
    }
    if !root.is_dir() {
        bail!(
            "Launcher target must be a directory, got: {}",
            root.display()
        );
    }

    let launcher_path = root.join(LAUNCHER_FILE_NAME);
    if launcher_path.exists() {
        if !launcher_path.is_file() {
            bail!(
                "Refusing to replace non-file path at {}",
                launcher_path.display()
            );
        }
        if !is_ccp_launcher_file(&launcher_path)? {
            bail!(
                "Refusing to overwrite unrelated existing file {}",
                launcher_path.display()
            );
        }
        if !force {
            bail!(
                "Launcher already exists at {}. Re-run with --force to replace it.",
                launcher_path.display()
            );
        }
    }

    fs::write(&launcher_path, build_linux_desktop_entry())
        .with_context(|| format!("Failed to write launcher {}", launcher_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&launcher_path)
            .with_context(|| format!("Failed to read metadata for {}", launcher_path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&launcher_path, permissions).with_context(|| {
            format!(
                "Failed to set executable permissions on {}",
                launcher_path.display()
            )
        })?;
    }

    Ok(launcher_path)
}

fn is_ccp_launcher_file(path: &Path) -> Result<bool> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read existing launcher {}", path.display()))?;
    Ok(content.contains(LAUNCHER_MARKER))
}

fn build_linux_desktop_entry() -> String {
    [
        "[Desktop Entry]",
        "Version=1.0",
        "Type=Application",
        "Name=CCP Copy Snapshot",
        "Comment=Copy project snapshot to clipboard using ccp -c",
        "Terminal=true",
        "Exec=sh -lc 'desktop_file=\"$1\"; case \"$desktop_file\" in file://*) desktop_file=\"${desktop_file#file://}\";; esac; launcher_dir=\"$(dirname -- \"$desktop_file\")\"; cd -- \"$launcher_dir\"; if ! command -v ccp >/dev/null 2>&1; then echo \"ccp executable not found in PATH.\"; echo \"Install with: cargo install ccp_tree --features clipboard\"; echo \"Press Enter to close...\"; read -r _; exit 127; fi; if ! ccp -c; then status=$?; echo \"ccp failed with status ${status}.\"; echo \"Press Enter to close...\"; read -r _; exit \"${status}\"; fi' sh %k",
        "Icon=utilities-terminal",
        LAUNCHER_MARKER,
        "",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ccp-launcher-{name}-{}", std::process::id()))
    }

    #[test]
    fn desktop_entry_invokes_ccp_clipboard_from_launcher_directory() {
        let entry = build_linux_desktop_entry();

        assert!(entry.contains("ccp -c"));
        assert!(entry.contains("%k"));
        assert!(entry.contains("dirname -- \"$desktop_file\""));
        assert!(entry.contains("command -v ccp"));
        assert!(entry.contains("Terminal=true"));
        assert!(entry.contains(LAUNCHER_MARKER));
    }

    #[test]
    fn create_launcher_in_specified_directory() {
        let root = test_root("specified");
        fs::create_dir_all(&root).expect("test root should exist");

        let path = create_launcher_for_environment(&root, false, true, true)
            .expect("launcher should be created");

        assert_eq!(path, root.join(LAUNCHER_FILE_NAME));
        assert!(path.exists());

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn create_launcher_refuses_existing_without_force() {
        let root = test_root("existing");
        fs::create_dir_all(&root).expect("test root should exist");
        let launcher = root.join(LAUNCHER_FILE_NAME);
        fs::write(&launcher, build_linux_desktop_entry()).expect("launcher should be written");

        let error = create_launcher_for_environment(&root, false, true, true)
            .expect_err("existing launcher should require --force");

        assert!(error.to_string().contains("Re-run with --force"));

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn create_launcher_does_not_overwrite_unrelated_file_even_with_force() {
        let root = test_root("unrelated");
        fs::create_dir_all(&root).expect("test root should exist");
        let launcher = root.join(LAUNCHER_FILE_NAME);
        fs::write(&launcher, "not-a-ccp-launcher").expect("file should be written");

        let error = create_launcher_for_environment(&root, true, true, true)
            .expect_err("unrelated file should not be overwritten");

        assert!(error.to_string().contains("unrelated existing file"));

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn create_launcher_replaces_existing_ccp_launcher_with_force() {
        let root = test_root("force");
        fs::create_dir_all(&root).expect("test root should exist");
        let launcher = root.join(LAUNCHER_FILE_NAME);
        fs::write(&launcher, build_linux_desktop_entry()).expect("launcher should be written");

        let path = create_launcher_for_environment(&root, true, true, true)
            .expect("existing launcher should be replaced with force");

        assert_eq!(path, launcher);
        assert!(fs::read_to_string(path)
            .expect("launcher should be readable")
            .contains(LAUNCHER_MARKER));

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn launcher_creation_reports_clipboard_prerequisite() {
        let root = test_root("clipboard-disabled");
        fs::create_dir_all(&root).expect("test root should exist");

        let error = create_launcher_for_environment(&root, false, true, false)
            .expect_err("clipboard-disabled build should fail");

        assert!(error.to_string().contains("--features clipboard"));

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn launcher_creation_reports_unsupported_platform() {
        let root = test_root("unsupported-platform");
        fs::create_dir_all(&root).expect("test root should exist");

        let error = create_launcher_for_environment(&root, false, false, true)
            .expect_err("non-linux platforms should fail");

        assert!(error.to_string().contains("supported only on Linux"));

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn launcher_creation_handles_paths_with_spaces() {
        let root = std::env::temp_dir().join(format!("ccp launcher spaced {}", std::process::id()));
        fs::create_dir_all(&root).expect("test root should exist");

        let path = create_launcher_for_environment(&root, false, true, true)
            .expect("launcher should be created");
        let entry = fs::read_to_string(&path).expect("launcher should be readable");

        assert!(entry.contains("%k"));
        assert!(entry.contains("dirname -- \"$desktop_file\""));

        fs::remove_dir_all(root).expect("test root should be removed");
    }
}
