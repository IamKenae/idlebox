use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Clone, Copy)]
pub enum FollowSymlinks {
    Yes,
    No,
}

pub struct ReplacementWarning {
    pub backup_path: PathBuf,
    pub error: io::Error,
}

pub fn same_file(left: &Path, right: &Path, follow: FollowSymlinks) -> io::Result<bool> {
    let left_identity = file_identity(left, follow)?;
    match file_identity(right, follow) {
        Ok(right_identity) => Ok(left_identity == right_identity),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

pub fn unique_sibling_path(path: &Path, purpose: &str) -> io::Result<PathBuf> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().unwrap_or_else(|| OsStr::new("file"));

    for attempt in 0..1000 {
        let mut candidate_name = OsString::from(".");
        candidate_name.push(file_name);
        candidate_name.push(format!(
            ".idlebox-{}-{}-{}",
            process::id(),
            purpose,
            attempt
        ));
        let candidate = parent.join(candidate_name);

        match fs::symlink_metadata(&candidate) {
            Ok(_) => continue,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(candidate),
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "could not allocate a temporary path next to {}",
            path.display()
        ),
    ))
}

#[cfg(unix)]
pub fn replace_file(
    staged_path: &Path,
    destination: &Path,
) -> io::Result<Option<ReplacementWarning>> {
    match fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.file_type().is_dir() => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "cannot replace {} because it is a directory",
                    destination.display()
                ),
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    fs::rename(staged_path, destination)?;
    Ok(None)
}

#[cfg(windows)]
pub fn replace_file(
    staged_path: &Path,
    destination: &Path,
) -> io::Result<Option<ReplacementWarning>> {
    let existing = match fs::symlink_metadata(destination) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };

    let Some(existing) = existing else {
        fs::rename(staged_path, destination)?;
        return Ok(None);
    };

    if existing.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "cannot replace {} because it is a directory",
                destination.display()
            ),
        ));
    }

    let backup_path = unique_sibling_path(destination, "old")?;
    fs::rename(destination, &backup_path)?;

    if let Err(replace_error) = fs::rename(staged_path, destination) {
        return match fs::rename(&backup_path, destination) {
            Ok(()) => Err(replace_error),
            Err(rollback_error) => Err(io::Error::other(format!(
                "failed to replace {}; rollback from {} also failed: {}",
                destination.display(),
                backup_path.display(),
                rollback_error
            ))),
        };
    }

    Ok(fs::remove_file(&backup_path)
        .err()
        .map(|error| ReplacementWarning { backup_path, error }))
}

#[cfg(not(any(unix, windows)))]
pub fn replace_file(
    staged_path: &Path,
    destination: &Path,
) -> io::Result<Option<ReplacementWarning>> {
    if fs::symlink_metadata(destination).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "replacing existing files is not supported on this platform",
        ));
    }
    fs::rename(staged_path, destination)?;
    Ok(None)
}

#[cfg(unix)]
fn file_identity(path: &Path, follow: FollowSymlinks) -> io::Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;

    let metadata = match follow {
        FollowSymlinks::Yes => fs::metadata(path)?,
        FollowSymlinks::No => fs::symlink_metadata(path)?,
    };
    Ok((metadata.dev(), metadata.ino()))
}

#[cfg(windows)]
fn file_identity(path: &Path, follow: FollowSymlinks) -> io::Result<(u32, u64)> {
    windows::file_identity(path, follow)
}

#[cfg(not(any(unix, windows)))]
fn file_identity(path: &Path, follow: FollowSymlinks) -> io::Result<PathBuf> {
    match follow {
        FollowSymlinks::Yes => fs::canonicalize(path),
        FollowSymlinks::No => {
            let absolute = if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()?.join(path)
            };
            let parent = absolute.parent().unwrap_or_else(|| Path::new("."));
            let file_name = absolute.file_name().unwrap_or_else(|| OsStr::new(""));
            Ok(fs::canonicalize(parent)?.join(file_name))
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::FollowSymlinks;
    use std::ffi::c_void;
    use std::io;
    use std::mem::MaybeUninit;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    type Handle = *mut c_void;

    const INVALID_HANDLE_VALUE: Handle = -1_isize as Handle;
    const FILE_READ_ATTRIBUTES: u32 = 0x0080;
    const FILE_SHARE_READ: u32 = 0x0001;
    const FILE_SHARE_WRITE: u32 = 0x0002;
    const FILE_SHARE_DELETE: u32 = 0x0004;
    const OPEN_EXISTING: u32 = 3;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

    #[repr(C)]
    struct FileTime {
        low_date_time: u32,
        high_date_time: u32,
    }

    #[repr(C)]
    struct ByHandleFileInformation {
        file_attributes: u32,
        creation_time: FileTime,
        last_access_time: FileTime,
        last_write_time: FileTime,
        volume_serial_number: u32,
        file_size_high: u32,
        file_size_low: u32,
        number_of_links: u32,
        file_index_high: u32,
        file_index_low: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        #[link_name = "CreateFileW"]
        fn create_file_w(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *mut c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: Handle,
        ) -> Handle;

        #[link_name = "GetFileInformationByHandle"]
        fn get_file_information_by_handle(
            file: Handle,
            information: *mut ByHandleFileInformation,
        ) -> i32;

        #[link_name = "CloseHandle"]
        fn close_handle(handle: Handle) -> i32;
    }

    pub fn file_identity(path: &Path, follow: FollowSymlinks) -> io::Result<(u32, u64)> {
        let wide_path = wide_path(path)?;
        let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
        if matches!(follow, FollowSymlinks::No) {
            flags |= FILE_FLAG_OPEN_REPARSE_POINT;
        }

        let handle = unsafe {
            create_file_w(
                wide_path.as_ptr(),
                FILE_READ_ATTRIBUTES,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                flags,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }

        let mut information = MaybeUninit::<ByHandleFileInformation>::uninit();
        let result = unsafe { get_file_information_by_handle(handle, information.as_mut_ptr()) };
        let error = (result == 0).then(io::Error::last_os_error);
        unsafe {
            close_handle(handle);
        }
        if let Some(error) = error {
            return Err(error);
        }

        let information = unsafe { information.assume_init() };
        let file_index =
            (u64::from(information.file_index_high) << 32) | u64::from(information.file_index_low);
        Ok((information.volume_serial_number, file_index))
    }

    fn wide_path(path: &Path) -> io::Result<Vec<u16>> {
        let mut path: Vec<u16> = path.as_os_str().encode_wide().collect();
        if path.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "path contains a null character",
            ));
        }
        path.push(0);
        Ok(path)
    }
}
