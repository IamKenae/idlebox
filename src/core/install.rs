#[cfg(any(unix, windows))]
use std::env;
#[cfg(any(unix, windows))]
use std::fs;
#[cfg(all(unix, not(windows)))]
use std::io;
#[cfg(windows)]
use std::io::{self, BufRead, BufReader};
#[cfg(any(unix, windows))]
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(any(unix, windows))]
use crate::core::{
    file_ops::{replace_file, same_file, unique_sibling_path, FollowSymlinks},
    Dispatcher,
};

pub struct InstallOptions<'a> {
    pub target: Option<&'a str>,
    pub force: bool,
    pub dry_run: bool,
}

pub fn install(options: InstallOptions<'_>) -> Result<i32, Box<dyn std::error::Error>> {
    #[cfg(not(any(unix, windows)))]
    {
        let _ = options;
        eprintln!("idlebox: --install is only supported on Unix-like systems and Windows");
        return Err("unsupported platform".into());
    }

    #[cfg(any(unix, windows))]
    {
        let dest_dir = match options.target {
            Some(path) => PathBuf::from(path),
            None => default_install_dir()?,
        };
        validate_destination(&dest_dir)?;

        let self_exe = env::current_exe()?;
        let self_path = self_exe.canonicalize().unwrap_or(self_exe);
        let inspection_source = launcher_source(&self_path, &dest_dir);

        let dispatcher = Dispatcher::new();
        let applet_names = dispatcher.applet_names();
        let mut launchers = Vec::with_capacity(applet_names.len());
        let mut conflicts = Vec::new();
        let mut counts = InstallCounts::default();

        for name in applet_names {
            let launcher_name = format!("{}{}", name, env::consts::EXE_SUFFIX);
            let launcher_path = dest_dir.join(launcher_name);

            match classify_launcher(&inspection_source, &launcher_path, options.force)? {
                LauncherState::Planned(action) => {
                    counts.add(action);
                    launchers.push(PlannedLauncher {
                        path: launcher_path,
                        action,
                    });
                }
                LauncherState::Conflict(kind) => conflicts.push(InstallConflict {
                    path: launcher_path,
                    kind,
                }),
            }
        }

        if options.dry_run {
            println!(
                "Previewing IdleBox applet installation to {}...",
                dest_dir.display()
            );
            for launcher in &launchers {
                match launcher.action {
                    InstallAction::Install => {
                        println!("  Would install: {}", launcher.path.display())
                    }
                    InstallAction::Update => {
                        println!("  Would update: {}", launcher.path.display())
                    }
                    InstallAction::Skip => {
                        println!("  Already installed: {}", launcher.path.display())
                    }
                }
            }
        }

        if !conflicts.is_empty() {
            print_conflicts(&conflicts);
            if options.dry_run {
                println!(
                    "Dry run: {} to install, {} to update, {} already installed, {} conflicts; no changes made.",
                    counts.install,
                    counts.update,
                    counts.skip,
                    conflicts.len()
                );
            } else {
                eprintln!(
                    "idlebox: preflight found {} conflict(s); no launchers were changed",
                    conflicts.len()
                );
            }

            let hint = if options.force {
                "resolve the launcher conflicts listed above"
            } else {
                "resolve the launcher conflicts listed above, or use --force to replace existing files"
            };
            return Err(hint.into());
        }

        if options.dry_run {
            println!(
                "Dry run: {} to install, {} to update, {} already installed; no changes made.",
                counts.install, counts.update, counts.skip
            );
            print_path_hint(&dest_dir);
            return Ok(0);
        }

        fs::create_dir_all(&dest_dir)?;
        let launcher_source = launcher_source(&self_path, &dest_dir);

        println!("Installing IdleBox applets to {}...", dest_dir.display());

        for launcher in launchers {
            let (action, replace_existing) = match launcher.action {
                InstallAction::Install => ("Installed", false),
                InstallAction::Update => ("Updated", true),
                InstallAction::Skip => continue,
            };
            let method = install_launcher(&launcher_source, &launcher.path, replace_existing)?;
            println!(
                "  {}: {} ({})",
                action,
                launcher.path.display(),
                method.label()
            );
        }

        println!(
            "Done. {} installed, {} updated, {} already installed.",
            counts.install, counts.update, counts.skip
        );
        print_path_hint(&dest_dir);
        Ok(0)
    }
}

#[cfg(any(unix, windows))]
#[derive(Clone, Copy)]
enum InstallAction {
    Install,
    Update,
    Skip,
}

#[cfg(any(unix, windows))]
struct PlannedLauncher {
    path: PathBuf,
    action: InstallAction,
}

#[cfg(any(unix, windows))]
enum LauncherState {
    Planned(InstallAction),
    Conflict(ConflictKind),
}

#[cfg(any(unix, windows))]
#[derive(Clone, Copy)]
enum ConflictKind {
    Directory,
    Existing,
}

#[cfg(any(unix, windows))]
struct InstallConflict {
    path: PathBuf,
    kind: ConflictKind,
}

#[cfg(any(unix, windows))]
#[derive(Default)]
struct InstallCounts {
    install: usize,
    update: usize,
    skip: usize,
}

#[cfg(any(unix, windows))]
impl InstallCounts {
    fn add(&mut self, action: InstallAction) {
        match action {
            InstallAction::Install => self.install += 1,
            InstallAction::Update => self.update += 1,
            InstallAction::Skip => self.skip += 1,
        }
    }
}

#[cfg(any(unix, windows))]
fn validate_destination(path: &Path) -> io::Result<()> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            format!("install target {} is not a directory", path.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => match fs::symlink_metadata(path) {
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                format!(
                    "install target {} is not a usable directory",
                    path.display()
                ),
            )),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    }
}

#[cfg(any(unix, windows))]
fn classify_launcher(source: &Path, launcher: &Path, force: bool) -> io::Result<LauncherState> {
    let metadata = match fs::symlink_metadata(launcher) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(LauncherState::Planned(InstallAction::Install));
        }
        Err(error) => return Err(error),
    };

    if metadata.file_type().is_dir() {
        return Ok(LauncherState::Conflict(ConflictKind::Directory));
    }
    if is_current_launcher(source, launcher)? {
        return Ok(LauncherState::Planned(InstallAction::Skip));
    }
    if force {
        Ok(LauncherState::Planned(InstallAction::Update))
    } else {
        Ok(LauncherState::Conflict(ConflictKind::Existing))
    }
}

#[cfg(any(unix, windows))]
fn is_current_launcher(source: &Path, launcher: &Path) -> io::Result<bool> {
    if same_file(source, launcher, FollowSymlinks::Yes)? {
        return Ok(true);
    }

    #[cfg(windows)]
    {
        match fs::metadata(launcher) {
            Ok(metadata) if metadata.is_file() => files_have_same_contents(source, launcher),
            Ok(_) => Ok(false),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    #[cfg(unix)]
    {
        Ok(false)
    }
}

#[cfg(windows)]
fn files_have_same_contents(left: &Path, right: &Path) -> io::Result<bool> {
    if fs::metadata(left)?.len() != fs::metadata(right)?.len() {
        return Ok(false);
    }

    let mut left = BufReader::new(fs::File::open(left)?);
    let mut right = BufReader::new(fs::File::open(right)?);

    loop {
        let left_bytes = left.fill_buf()?;
        let right_bytes = right.fill_buf()?;
        let compared = left_bytes.len().min(right_bytes.len());

        if left_bytes[..compared] != right_bytes[..compared] {
            return Ok(false);
        }
        if compared == 0 {
            return Ok(left_bytes.is_empty() && right_bytes.is_empty());
        }

        left.consume(compared);
        right.consume(compared);
    }
}

#[cfg(any(unix, windows))]
fn print_conflicts(conflicts: &[InstallConflict]) {
    for conflict in conflicts {
        let reason = match conflict.kind {
            ConflictKind::Directory => "is a directory; directories are never replaced",
            ConflictKind::Existing => "already exists and is not the current IdleBox launcher",
        };
        eprintln!("  Conflict: {} ({})", conflict.path.display(), reason);
    }
}

#[cfg(any(unix, windows))]
fn print_path_hint(dest_dir: &Path) {
    let in_path = env::var_os("PATH")
        .map(|path| env::split_paths(&path).any(|entry| paths_match(&entry, dest_dir)))
        .unwrap_or(false);

    if !in_path {
        println!(
            "Tip: add {} to PATH to invoke installed applets by name.",
            dest_dir.display()
        );
    }
}

#[cfg(any(unix, windows))]
fn paths_match(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (absolute_or_canonical(left), absolute_or_canonical(right)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

#[cfg(any(unix, windows))]
fn absolute_or_canonical(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok().or_else(|| {
        if path.is_absolute() {
            Some(path.to_path_buf())
        } else {
            env::current_dir().ok().map(|current| current.join(path))
        }
    })
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
    replace_existing: bool,
) -> Result<InstallMethod, Box<dyn std::error::Error>> {
    let staged_path = unique_sibling_path(launcher_path, "new")?;
    let method = create_launcher(source, &staged_path)?;

    if !replace_existing {
        match fs::symlink_metadata(launcher_path) {
            Ok(_) => {
                let _ = fs::remove_file(&staged_path);
                return Err(format!(
                    "{} appeared after preflight; rerun the installation",
                    launcher_path.display()
                )
                .into());
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                let _ = fs::remove_file(&staged_path);
                return Err(error.into());
            }
        }
    }

    let warning = match replace_file(&staged_path, launcher_path) {
        Ok(warning) => warning,
        Err(error) => {
            let _ = fs::remove_file(&staged_path);
            return Err(error.into());
        }
    };
    if let Some(warning) = warning {
        eprintln!(
            "idlebox: warning: launcher was installed, but old backup {} could not be removed: {}",
            warning.backup_path.display(),
            warning.error
        );
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
