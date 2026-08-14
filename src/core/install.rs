#[cfg(unix)]
use std::env;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(unix)]
use crate::core::Dispatcher;

pub fn install(target: Option<&str>) -> Result<i32, Box<dyn std::error::Error>> {
    #[cfg(not(unix))]
    {
        let _ = target;
        eprintln!("idlebox: --install is only supported on Unix-like systems");
        return Err("unsupported platform".into());
    }

    #[cfg(unix)]
    {
        let dest_dir = match target {
            Some(p) => PathBuf::from(p),
            None => default_install_dir(),
        };

        fs::create_dir_all(&dest_dir)?;

        let self_exe = env::current_exe()?;
        let self_path = self_exe.canonicalize().unwrap_or(self_exe.clone());
        let self_display = self_exe
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("idlebox");

        let dispatcher = Dispatcher::new();
        let applet_names = dispatcher.applet_names();

        println!("Installing IdleBox applets to {}...", dest_dir.display());

        for name in &applet_names {
            let link_path = dest_dir.join(name);

            if link_path.exists() || link_path.symlink_metadata().is_ok() {
                fs::remove_file(&link_path)?;
            }

            let link_target = if dest_dir == self_exe.parent().unwrap_or(Path::new("")) {
                Path::new(self_display)
            } else {
                &self_path
            };

            symlink(link_target, &link_path)?;
            println!(
                "  Created symlink: {} -> {}",
                link_path.display(),
                link_target.display()
            );
        }

        println!("Done. {} applets installed.", applet_names.len());
        Ok(0)
    }
}

#[cfg(unix)]
fn default_install_dir() -> PathBuf {
    PathBuf::from("/usr/local/bin")
}
