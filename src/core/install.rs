#[cfg(any(unix, windows))]
use std::env;
#[cfg(any(unix, windows))]
use std::fs;
#[cfg(any(unix, windows))]
use std::path::{Path, PathBuf};
#[cfg(any(unix, windows))]
use std::process;

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(any(unix, windows))]
use crate::core::Dispatcher;

pub fn install(target: Option<&str>) -> Result<i32, Box<dyn std::error::Error>> {
    #[cfg(not(any(unix, windows)))]
    {
        let _ = target;
        eprintln!("idlebox: --install is only supported on Unix-like systems and Windows");
        return Err("unsupported platform".into());
    }

    #[cfg(any(unix, windows))]
    {
        let dest_dir = match target {
            Some(path) => PathBuf::from(path),
            None => default_install_dir()?,
        };

        fs::create_dir_all(&dest_dir)?;

        let self_exe = env::current_exe()?;
        let self_path = self_exe.canonicalize().unwrap_or(self_exe);
        let launcher_source = launcher_source(&self_path, &dest_dir);

        let dispatcher = Dispatcher::new();
        let applet_names = dispatcher.applet_names();

        println!("Installing IdleBox applets to {}...", dest_dir.display());

        for name in &applet_names {
            let launcher_name = format!("{}{}", name, env::consts::EXE_SUFFIX);
            let launcher_path = dest_dir.join(launcher_name);
            let method = install_launcher(&launcher_source, &launcher_path)?;

            println!(
                "  Installed: {} ({})",
                launcher_path.display(),
                method.label()
            );
        }

        println!("Done. {} applets installed.", applet_names.len());
        Ok(0)
    }
}

#[cfg(any(unix, windows))]
#[derive(Clone, Copy)]
enum InstallMethod {
    #[cfg(unix)]
    Symlink,
    #[cfg(windows)]
    HardLink,
    #[cfg(windows)]
    Copy,
}

#[cfg(any(unix, windows))]
impl InstallMethod {
    fn label(self) -> &'static str {
        match self {
            #[cfg(unix)]
            Self::Symlink => "symbolic link",
            #[cfg(windows)]
            Self::HardLink => "hard link",
            #[cfg(windows)]
            Self::Copy => "copy",
        }
    }
}

#[cfg(any(unix, windows))]
fn install_launcher(
    source: &Path,
    launcher_path: &Path,
) -> Result<InstallMethod, Box<dyn std::error::Error>> {
    let staged_path = unique_sibling_path(launcher_path, "new")?;
    let method = create_launcher(source, &staged_path)?;

    if let Err(error) = replace_launcher(&staged_path, launcher_path) {
        let _ = fs::remove_file(&staged_path);
        return Err(error);
    }

    Ok(method)
}

#[cfg(unix)]
fn create_launcher(
    source: &Path,
    staged_path: &Path,
) -> Result<InstallMethod, Box<dyn std::error::Error>> {
    symlink(source, staged_path)?;
    Ok(InstallMethod::Symlink)
}

#[cfg(windows)]
fn create_launcher(
    source: &Path,
    staged_path: &Path,
) -> Result<InstallMethod, Box<dyn std::error::Error>> {
    match fs::hard_link(source, staged_path) {
        Ok(()) => Ok(InstallMethod::HardLink),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Err(error.into()),
        Err(hard_link_error) => match copy_exclusive(source, staged_path) {
            Ok(()) => Ok(InstallMethod::Copy),
            Err(copy_error) => Err(format!(
                "failed to create launcher {}: hard link: {}; copy: {}",
                staged_path.display(),
                hard_link_error,
                copy_error
            )
            .into()),
        },
    }
}

#[cfg(windows)]
fn copy_exclusive(source: &Path, destination: &Path) -> Result<(), std::io::Error> {
    let mut source_file = fs::File::open(source)?;
    let mut destination_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;

    match std::io::copy(&mut source_file, &mut destination_file) {
        Ok(_) => Ok(()),
        Err(error) => {
            drop(destination_file);
            let _ = fs::remove_file(destination);
            Err(error)
        }
    }
}

#[cfg(unix)]
fn replace_launcher(
    staged_path: &Path,
    launcher_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    match fs::symlink_metadata(launcher_path) {
        Ok(metadata) if metadata.file_type().is_dir() => {
            return Err(format!(
                "cannot replace launcher {} because it is a directory",
                launcher_path.display()
            )
            .into());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    fs::rename(staged_path, launcher_path)?;
    Ok(())
}

#[cfg(windows)]
fn replace_launcher(
    staged_path: &Path,
    launcher_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let existing = match fs::symlink_metadata(launcher_path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };

    let Some(existing) = existing else {
        fs::rename(staged_path, launcher_path)?;
        return Ok(());
    };

    if existing.file_type().is_dir() {
        return Err(format!(
            "cannot replace launcher {} because it is a directory",
            launcher_path.display()
        )
        .into());
    }

    let backup_path = unique_sibling_path(launcher_path, "old")?;
    fs::rename(launcher_path, &backup_path)?;

    if let Err(install_error) = fs::rename(staged_path, launcher_path) {
        return match fs::rename(&backup_path, launcher_path) {
            Ok(()) => Err(install_error.into()),
            Err(rollback_error) => Err(format!(
                "failed to install launcher {}: {}; rollback from {} also failed: {}",
                launcher_path.display(),
                install_error,
                backup_path.display(),
                rollback_error
            )
            .into()),
        };
    }

    if let Err(error) = fs::remove_file(&backup_path) {
        eprintln!(
            "idlebox: warning: launcher was installed, but old backup {} could not be removed: {}",
            backup_path.display(),
            error
        );
    }

    Ok(())
}

#[cfg(any(unix, windows))]
fn unique_sibling_path(
    launcher_path: &Path,
    purpose: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let parent = launcher_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = launcher_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("applet");

    for attempt in 0..1000 {
        let candidate = parent.join(format!(
            ".{}.idlebox-{}-{}-{}",
            file_name,
            process::id(),
            purpose,
            attempt
        ));

        match fs::symlink_metadata(&candidate) {
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(candidate),
            Err(error) => return Err(error.into()),
        }
    }

    Err(format!(
        "could not allocate a temporary path next to {}",
        launcher_path.display()
    )
    .into())
}

#[cfg(unix)]
fn launcher_source(self_path: &Path, dest_dir: &Path) -> PathBuf {
    let canonical_dest = dest_dir
        .canonicalize()
        .unwrap_or_else(|_| dest_dir.to_path_buf());

    if self_path.parent() == Some(canonical_dest.as_path()) {
        self_path
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| self_path.to_path_buf())
    } else {
        self_path.to_path_buf()
    }
}

#[cfg(windows)]
fn launcher_source(self_path: &Path, _dest_dir: &Path) -> PathBuf {
    self_path.to_path_buf()
}

#[cfg(unix)]
fn default_install_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from("/usr/local/bin"))
}

#[cfg(windows)]
fn default_install_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
        return Ok(PathBuf::from(local_app_data).join("IdleBox").join("bin"));
    }

    if let Some(user_profile) = env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(user_profile).join(".local").join("bin"));
    }

    Err("could not determine the Windows install directory; pass PATH to --install".into())
}
