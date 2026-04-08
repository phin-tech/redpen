use std::fs;
use std::path::PathBuf;

fn bundled_cli_path() -> Result<PathBuf, String> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("redpen")))
        .ok_or("Could not find redpen next to app binary".to_string())
}

fn installed_cli_path() -> Result<PathBuf, String> {
    Ok(dirs::home_dir()
        .ok_or("Could not determine home directory".to_string())?
        .join(".local")
        .join("bin")
        .join("redpen"))
}

pub fn cli_is_installed() -> bool {
    installed_cli_path().map(|p| p.exists()).unwrap_or(false)
}

pub fn install_cli_binary() -> Result<String, String> {
    let target = installed_cli_path()?;
    let source = bundled_cli_path()?;

    if !source.exists() {
        return Err(format!("Bundled redpen binary not found at {}", source.display()));
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
    }

    let should_copy = if target.exists() {
        let src_modified = fs::metadata(&source).and_then(|m| m.modified()).ok();
        let dst_modified = fs::metadata(&target).and_then(|m| m.modified()).ok();
        match (src_modified, dst_modified) {
            (Some(src), Some(dst)) => src > dst,
            _ => true,
        }
    } else {
        true
    };

    if should_copy {
        fs::copy(&source, &target)
            .map_err(|e| format!("Failed to copy redpen CLI to {}: {}", target.display(), e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&target).map_err(|e| e.to_string())?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&target, perms).map_err(|e| e.to_string())?;
        }
    }

    Ok(target.to_string_lossy().to_string())
}
