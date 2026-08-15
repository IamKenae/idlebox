use crate::core::{
    file_ops::{replace_file, unique_sibling_path},
    Applet,
};
use flate2::read::DeflateDecoder;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const LOCAL_HEADER: u32 = 0x0403_4b50;
const CENTRAL_HEADER: u32 = 0x0201_4b50;
const END_OF_CENTRAL_DIRECTORY: u32 = 0x0605_4b50;
const ZIP64_EXTRA: u16 = 0x0001;
const MAX_EOCD_SIZE: u64 = 22 + u16::MAX as u64;

pub struct UnzipApplet;

impl Applet for UnzipApplet {
    fn name(&self) -> &'static str {
        "unzip"
    }

    fn description(&self) -> &'static str {
        "List or extract files from ZIP archives"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let options = match Options::parse(args) {
            Ok(options) => options,
            Err(error) => {
                eprintln!("unzip: {}", error);
                return Ok(1);
            }
        };

        match run_unzip(&options) {
            Ok(()) => Ok(0),
            Err(error) => {
                eprintln!("unzip: {}", error);
                Ok(1)
            }
        }
    }

    fn help(&self) {
        println!("Usage: unzip [OPTION]... ARCHIVE");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -l, --list          List archive contents without extracting");
        println!("  -o, --overwrite     Overwrite existing files");
        println!("  -d DIR              Extract files into DIR");
    }
}

struct Options {
    archive: PathBuf,
    destination: PathBuf,
    list: bool,
    overwrite: bool,
}

impl Options {
    fn parse(args: &[String]) -> io::Result<Self> {
        let mut destination = PathBuf::from(".");
        let mut list = false;
        let mut overwrite = false;
        let mut operands = Vec::new();
        let mut options_ended = false;
        let mut index = 0;

        while index < args.len() {
            let argument = args[index].as_str();
            if options_ended {
                operands.push(argument);
                index += 1;
                continue;
            }

            match argument {
                "--" => options_ended = true,
                "--list" => list = true,
                "--overwrite" => overwrite = true,
                "--directory" => {
                    index += 1;
                    let value = args.get(index).ok_or_else(|| {
                        invalid_input("option '--directory' requires an argument")
                    })?;
                    destination = PathBuf::from(value);
                }
                _ if argument.starts_with("--directory=") => {
                    let value = &argument["--directory=".len()..];
                    if value.is_empty() {
                        return Err(invalid_input("option '--directory' requires an argument"));
                    }
                    destination = PathBuf::from(value);
                }
                _ if argument.starts_with("--") => {
                    return Err(invalid_input(format!("unrecognized option '{}'", argument)));
                }
                _ if argument.starts_with('-') && argument != "-" => {
                    for (offset, flag) in argument[1..].char_indices() {
                        match flag {
                            'l' => list = true,
                            'o' => overwrite = true,
                            'd' => {
                                let value_start = 1 + offset + flag.len_utf8();
                                let attached = &argument[value_start..];
                                if !attached.is_empty() {
                                    let value = attached.trim_start_matches('=');
                                    if value.is_empty() {
                                        return Err(invalid_input(
                                            "option '-d' requires an argument",
                                        ));
                                    }
                                    destination = PathBuf::from(value);
                                } else {
                                    index += 1;
                                    let value = args.get(index).ok_or_else(|| {
                                        invalid_input("option '-d' requires an argument")
                                    })?;
                                    destination = PathBuf::from(value);
                                }
                                break;
                            }
                            _ => {
                                return Err(invalid_input(format!("invalid option -- '{}'", flag)));
                            }
                        }
                    }
                }
                _ => operands.push(argument),
            }
            index += 1;
        }

        if operands.is_empty() {
            return Err(invalid_input("missing archive operand"));
        }
        if operands.len() > 1 {
            return Err(invalid_input(format!(
                "unexpected operand '{}'",
                operands[1]
            )));
        }

        Ok(Self {
            archive: PathBuf::from(operands[0]),
            destination,
            list,
            overwrite,
        })
    }
}

struct ZipArchive {
    file: File,
    entries: Vec<ZipEntry>,
    central_offset: u64,
}

struct ZipEntry {
    name: String,
    raw_name: Vec<u8>,
    flags: u16,
    method: u16,
    modified_time: u16,
    modified_date: u16,
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    local_offset: u32,
    kind: EntryKind,
    unix_mode: Option<u32>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    File,
    Directory,
    Symlink,
    Special,
}

fn run_unzip(options: &Options) -> io::Result<()> {
    let mut archive = ZipArchive::open(&options.archive)?;
    if options.list {
        list_archive(&options.archive, &archive.entries);
        return Ok(());
    }

    validate_entry_paths(&archive.entries)?;
    ensure_extraction_root(&options.destination)?;
    for index in 0..archive.entries.len() {
        extract_entry(&mut archive, index, &options.destination, options.overwrite)?;
    }
    apply_directory_modes(&archive.entries, &options.destination)?;
    Ok(())
}

fn validate_entry_paths(entries: &[ZipEntry]) -> io::Result<()> {
    for entry in entries {
        let relative = safe_relative_path(&entry.name)?;
        if relative.as_os_str().is_empty() && entry.kind != EntryKind::Directory {
            return Err(invalid_data("ZIP entry has an empty file name"));
        }
        match entry.kind {
            EntryKind::Directory if entry.compressed_size != 0 || entry.uncompressed_size != 0 => {
                return Err(invalid_data(format!(
                    "directory entry '{}' contains file data",
                    entry.name
                )));
            }
            EntryKind::Symlink => {
                return Err(invalid_data(format!(
                    "refusing to extract symbolic link '{}'",
                    entry.name
                )));
            }
            EntryKind::Special => {
                return Err(invalid_data(format!(
                    "refusing to extract special file '{}'",
                    entry.name
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

impl ZipArchive {
    fn open(path: &Path) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let file_size = file.metadata()?.len();
        let eocd = find_eocd(&mut file, file_size)?;

        if eocd.disk != 0 || eocd.central_disk != 0 || eocd.disk_entries != eocd.entries {
            return Err(invalid_data("multi-disk ZIP archives are not supported"));
        }
        if eocd.entries == u16::MAX
            || eocd.central_size == u32::MAX
            || eocd.central_offset == u32::MAX
        {
            return Err(invalid_data("ZIP64 archives are not supported"));
        }

        let central_offset = u64::from(eocd.central_offset);
        let central_end = central_offset
            .checked_add(u64::from(eocd.central_size))
            .ok_or_else(|| invalid_data("central directory size overflows"))?;
        if central_end > eocd.offset {
            return Err(invalid_data("central directory is outside the archive"));
        }

        file.seek(SeekFrom::Start(central_offset))?;
        let mut entries = Vec::with_capacity(usize::from(eocd.entries));
        for _ in 0..eocd.entries {
            let entry = read_central_entry(&mut file, central_end)?;
            entries.push(entry);
        }
        if file.stream_position()? != central_end {
            return Err(invalid_data("central directory has an invalid size"));
        }

        Ok(Self {
            file,
            entries,
            central_offset,
        })
    }
}

struct EndRecord {
    offset: u64,
    disk: u16,
    central_disk: u16,
    disk_entries: u16,
    entries: u16,
    central_size: u32,
    central_offset: u32,
}

fn find_eocd(file: &mut File, file_size: u64) -> io::Result<EndRecord> {
    if file_size < 22 {
        return Err(invalid_data("not a ZIP archive (end record not found)"));
    }

    let search_size = file_size.min(MAX_EOCD_SIZE);
    let search_offset = file_size - search_size;
    let mut tail = vec![0_u8; search_size as usize];
    file.seek(SeekFrom::Start(search_offset))?;
    file.read_exact(&mut tail)?;

    for offset in (0..=tail.len() - 22).rev() {
        if le_u32(&tail[offset..offset + 4]) != END_OF_CENTRAL_DIRECTORY {
            continue;
        }
        let comment_length = usize::from(le_u16(&tail[offset + 20..offset + 22]));
        if offset + 22 + comment_length != tail.len() {
            continue;
        }
        return Ok(EndRecord {
            offset: search_offset + offset as u64,
            disk: le_u16(&tail[offset + 4..offset + 6]),
            central_disk: le_u16(&tail[offset + 6..offset + 8]),
            disk_entries: le_u16(&tail[offset + 8..offset + 10]),
            entries: le_u16(&tail[offset + 10..offset + 12]),
            central_size: le_u32(&tail[offset + 12..offset + 16]),
            central_offset: le_u32(&tail[offset + 16..offset + 20]),
        });
    }

    Err(invalid_data("not a ZIP archive (end record not found)"))
}

fn read_central_entry(file: &mut File, central_end: u64) -> io::Result<ZipEntry> {
    let start = file.stream_position()?;
    if start.checked_add(46).is_none_or(|end| end > central_end) {
        return Err(invalid_data("truncated central directory entry"));
    }

    let mut header = [0_u8; 46];
    file.read_exact(&mut header)?;
    if le_u32(&header[0..4]) != CENTRAL_HEADER {
        return Err(invalid_data("invalid central directory entry signature"));
    }

    let made_by = le_u16(&header[4..6]);
    let flags = le_u16(&header[8..10]);
    reject_encryption(flags)?;
    let method = le_u16(&header[10..12]);
    if method != 0 && method != 8 {
        return Err(invalid_data(format!(
            "unsupported ZIP compression method {}",
            method
        )));
    }

    let compressed_size = le_u32(&header[20..24]);
    let uncompressed_size = le_u32(&header[24..28]);
    let name_length = usize::from(le_u16(&header[28..30]));
    let extra_length = usize::from(le_u16(&header[30..32]));
    let comment_length = usize::from(le_u16(&header[32..34]));
    let disk = le_u16(&header[34..36]);
    let external_attributes = le_u32(&header[38..42]);
    let local_offset = le_u32(&header[42..46]);

    if disk != 0 {
        return Err(invalid_data("multi-disk ZIP archives are not supported"));
    }
    if compressed_size == u32::MAX || uncompressed_size == u32::MAX || local_offset == u32::MAX {
        return Err(invalid_data("ZIP64 archives are not supported"));
    }

    let variable_size = name_length
        .checked_add(extra_length)
        .and_then(|size| size.checked_add(comment_length))
        .ok_or_else(|| invalid_data("central directory entry size overflows"))?;
    if start
        .checked_add(46 + variable_size as u64)
        .is_none_or(|end| end > central_end)
    {
        return Err(invalid_data("truncated central directory entry"));
    }

    let mut raw_name = vec![0_u8; name_length];
    file.read_exact(&mut raw_name)?;
    let mut extra = vec![0_u8; extra_length];
    file.read_exact(&mut extra)?;
    reject_zip64_extra(&extra)?;
    file.seek(SeekFrom::Current(comment_length as i64))?;

    let name = decode_name(&raw_name, flags)?;
    let kind = entry_kind(made_by, external_attributes, &name);
    let unix_mode = entry_unix_mode(made_by, external_attributes);
    Ok(ZipEntry {
        name,
        raw_name,
        flags,
        method,
        modified_time: le_u16(&header[12..14]),
        modified_date: le_u16(&header[14..16]),
        crc32: le_u32(&header[16..20]),
        compressed_size,
        uncompressed_size,
        local_offset,
        kind,
        unix_mode,
    })
}

fn reject_zip64_extra(extra: &[u8]) -> io::Result<()> {
    let mut offset = 0;
    while offset < extra.len() {
        if extra.len() - offset < 4 {
            return Err(invalid_data("malformed ZIP extra field"));
        }
        let identifier = le_u16(&extra[offset..offset + 2]);
        let size = usize::from(le_u16(&extra[offset + 2..offset + 4]));
        offset += 4;
        let end = offset
            .checked_add(size)
            .ok_or_else(|| invalid_data("ZIP extra field size overflows"))?;
        if end > extra.len() {
            return Err(invalid_data("malformed ZIP extra field"));
        }
        if identifier == ZIP64_EXTRA {
            return Err(invalid_data("ZIP64 archives are not supported"));
        }
        offset = end;
    }
    Ok(())
}

fn reject_encryption(flags: u16) -> io::Result<()> {
    const ENCRYPTED: u16 = (1 << 0) | (1 << 6) | (1 << 13);
    if flags & ENCRYPTED != 0 {
        Err(invalid_data("encrypted ZIP entries are not supported"))
    } else {
        Ok(())
    }
}

fn entry_kind(made_by: u16, external_attributes: u32, name: &str) -> EntryKind {
    const FILE_TYPE_MASK: u32 = 0o170000;
    const REGULAR_FILE: u32 = 0o100000;
    const DIRECTORY: u32 = 0o040000;
    const SYMLINK: u32 = 0o120000;

    let host = made_by >> 8;
    if host == 3 {
        match (external_attributes >> 16) & FILE_TYPE_MASK {
            SYMLINK => return EntryKind::Symlink,
            DIRECTORY => return EntryKind::Directory,
            REGULAR_FILE | 0 => {}
            _ => return EntryKind::Special,
        }
    }
    if name.ends_with('/') || external_attributes & 0x10 != 0 {
        EntryKind::Directory
    } else {
        EntryKind::File
    }
}

fn entry_unix_mode(made_by: u16, external_attributes: u32) -> Option<u32> {
    let host = made_by >> 8;
    let mode = external_attributes >> 16;

    // Some writers identify as Unix but leave the external attributes empty.
    // Treat an entirely absent mode as unspecified so normal creation defaults
    // and the process umask still apply. A nonzero mode with permission bits of
    // zero is distinct and deliberately restores mode 0000.
    (host == 3 && mode != 0).then_some(mode & 0o7777)
}

fn list_archive(path: &Path, entries: &[ZipEntry]) {
    println!("Archive:  {}", path.display());
    println!("  Length      Date    Time    Name");
    println!("---------  ---------- -----   ----");
    let mut total = 0_u64;
    for entry in entries {
        total += u64::from(entry.uncompressed_size);
        let year = 1980 + u32::from(entry.modified_date >> 9);
        let month = (entry.modified_date >> 5) & 0x0f;
        let day = entry.modified_date & 0x1f;
        let hour = entry.modified_time >> 11;
        let minute = (entry.modified_time >> 5) & 0x3f;
        println!(
            "{:9}  {:04}-{:02}-{:02} {:02}:{:02}   {}",
            entry.uncompressed_size, year, month, day, hour, minute, entry.name
        );
    }
    println!("---------                     -------");
    println!("{:9}                     {} files", total, entries.len());
}

fn extract_entry(
    archive: &mut ZipArchive,
    index: usize,
    root: &Path,
    overwrite: bool,
) -> io::Result<()> {
    let entry = &archive.entries[index];
    let relative = safe_relative_path(&entry.name)?;
    if relative.as_os_str().is_empty() {
        if entry.kind == EntryKind::Directory {
            return Ok(());
        }
        return Err(invalid_data("ZIP entry has an empty file name"));
    }
    let destination = root.join(&relative);

    match entry.kind {
        EntryKind::Directory => {
            if entry.compressed_size != 0 || entry.uncompressed_size != 0 {
                return Err(invalid_data(format!(
                    "directory entry '{}' contains file data",
                    entry.name
                )));
            }
            ensure_safe_directory_tree(root, &destination)
        }
        EntryKind::Symlink => Err(invalid_data(format!(
            "refusing to extract symbolic link '{}'",
            entry.name
        ))),
        EntryKind::Special => Err(invalid_data(format!(
            "refusing to extract special file '{}'",
            entry.name
        ))),
        EntryKind::File => {
            let parent = destination.parent().unwrap_or(root);
            ensure_safe_directory_tree(root, parent)?;
            extract_regular_file(
                archive,
                index,
                root,
                &destination,
                overwrite,
                entry.unix_mode,
            )
        }
    }
}

fn extract_regular_file(
    archive: &mut ZipArchive,
    index: usize,
    root: &Path,
    destination: &Path,
    overwrite: bool,
    unix_mode: Option<u32>,
) -> io::Result<()> {
    reject_unsafe_existing_target(destination, overwrite)?;

    let (output_path, staged) = if overwrite {
        (unique_sibling_path(destination, "unzip")?, true)
    } else {
        (destination.to_path_buf(), false)
    };
    let mut output = match create_output_file(&output_path, unix_mode) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists && !overwrite => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "{} already exists; use -o to overwrite",
                    destination.display()
                ),
            ));
        }
        Err(error) => return Err(error),
    };

    let result = extract_entry_data(archive, index, &mut output)
        .and_then(|()| output.flush())
        .and_then(|()| set_file_mode(&output, unix_mode));
    drop(output);
    if let Err(error) = result {
        let _ = fs::remove_file(&output_path);
        return Err(error);
    }

    if staged {
        let parent = destination.parent().unwrap_or(root);
        if let Err(error) = ensure_safe_directory_tree(root, parent)
            .and_then(|()| reject_unsafe_existing_target(destination, true))
        {
            let _ = fs::remove_file(&output_path);
            return Err(error);
        }
        match replace_file(&output_path, destination) {
            Ok(warning) => {
                if let Some(warning) = warning {
                    eprintln!(
                        "unzip: warning: extracted '{}', but old backup '{}' could not be removed: {}",
                        destination.display(),
                        warning.backup_path.display(),
                        warning.error
                    );
                }
            }
            Err(error) => {
                let _ = fs::remove_file(&output_path);
                return Err(error);
            }
        }
    }

    Ok(())
}

fn create_output_file(path: &Path, unix_mode: Option<u32>) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(not(unix))]
    let _ = unix_mode;
    #[cfg(unix)]
    if unix_mode.is_some() {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

fn apply_directory_modes(entries: &[ZipEntry], root: &Path) -> io::Result<()> {
    let mut directories = entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Directory)
        .filter_map(|entry| entry.unix_mode.map(|mode| (&entry.name, mode)))
        .map(|(name, mode)| safe_relative_path(name).map(|path| (root.join(path), mode)))
        .collect::<io::Result<Vec<_>>>()?;

    // Permissions such as 0555 or 0500 must not be installed while child
    // entries are still being created. Applying deepest paths first after the
    // whole archive succeeded preserves them without blocking extraction.
    directories.sort_by_key(|(path, _)| std::cmp::Reverse(path.components().count()));
    for (directory, mode) in directories {
        ensure_safe_directory_tree(root, &directory)?;
        let metadata = fs::symlink_metadata(&directory)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(invalid_data(format!(
                "refusing to set permissions on unsafe directory '{}'",
                directory.display()
            )));
        }
        set_path_mode(&directory, mode)?;
    }
    Ok(())
}

#[cfg(unix)]
fn set_file_mode(file: &File, mode: Option<u32>) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    if let Some(mode) = mode {
        file.set_permissions(fs::Permissions::from_mode(mode))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_file_mode(_file: &File, _mode: Option<u32>) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_path_mode(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn set_path_mode(_path: &Path, _mode: u32) -> io::Result<()> {
    Ok(())
}

fn extract_entry_data(archive: &mut ZipArchive, index: usize, output: &mut File) -> io::Result<()> {
    let entry = &archive.entries[index];
    let offset = u64::from(entry.local_offset);
    if offset
        .checked_add(30)
        .is_none_or(|end| end > archive.central_offset)
    {
        return Err(invalid_data(format!(
            "local header for '{}' is outside the archive",
            entry.name
        )));
    }

    archive.file.seek(SeekFrom::Start(offset))?;
    let mut header = [0_u8; 30];
    archive.file.read_exact(&mut header)?;
    if le_u32(&header[0..4]) != LOCAL_HEADER {
        return Err(invalid_data(format!(
            "invalid local header for '{}'",
            entry.name
        )));
    }

    let local_flags = le_u16(&header[6..8]);
    reject_encryption(local_flags)?;
    const LOCAL_CENTRAL_FLAGS: u16 = (1 << 3) | (1 << 11);
    if local_flags & LOCAL_CENTRAL_FLAGS != entry.flags & LOCAL_CENTRAL_FLAGS {
        return Err(invalid_data(format!(
            "local and central flags differ for '{}'",
            entry.name
        )));
    }
    if le_u16(&header[8..10]) != entry.method {
        return Err(invalid_data(format!(
            "compression method mismatch for '{}'",
            entry.name
        )));
    }
    if local_flags & (1 << 3) == 0
        && (le_u32(&header[14..18]) != entry.crc32
            || le_u32(&header[18..22]) != entry.compressed_size
            || le_u32(&header[22..26]) != entry.uncompressed_size)
    {
        return Err(invalid_data(format!(
            "local header size or checksum mismatch for '{}'",
            entry.name
        )));
    }

    let name_length = usize::from(le_u16(&header[26..28]));
    let extra_length = usize::from(le_u16(&header[28..30]));
    let data_offset = offset
        .checked_add(30)
        .and_then(|value| value.checked_add(name_length as u64))
        .and_then(|value| value.checked_add(extra_length as u64))
        .ok_or_else(|| invalid_data("local header size overflows"))?;
    let data_end = data_offset
        .checked_add(u64::from(entry.compressed_size))
        .ok_or_else(|| invalid_data("compressed data size overflows"))?;
    if data_end > archive.central_offset {
        return Err(invalid_data(format!(
            "compressed data for '{}' is outside the archive",
            entry.name
        )));
    }

    let mut local_name = vec![0_u8; name_length];
    archive.file.read_exact(&mut local_name)?;
    if local_name != entry.raw_name {
        return Err(invalid_data(format!(
            "local and central file names differ for '{}'",
            entry.name
        )));
    }
    let mut extra = vec![0_u8; extra_length];
    archive.file.read_exact(&mut extra)?;
    reject_zip64_extra(&extra)?;

    let expected_size = u64::from(entry.uncompressed_size);
    let (actual_size, actual_crc) = match entry.method {
        0 => {
            if entry.compressed_size != entry.uncompressed_size {
                return Err(invalid_data(format!(
                    "stored entry '{}' has inconsistent sizes",
                    entry.name
                )));
            }
            let mut input = (&mut archive.file).take(u64::from(entry.compressed_size));
            copy_and_checksum(&mut input, output, expected_size)?
        }
        8 => {
            let input = (&mut archive.file).take(u64::from(entry.compressed_size));
            let mut decoder = DeflateDecoder::new(input);
            let result = copy_and_checksum(&mut decoder, output, expected_size)?;
            if decoder.total_in() != u64::from(entry.compressed_size) {
                return Err(invalid_data(format!(
                    "compressed size mismatch for '{}'",
                    entry.name
                )));
            }
            result
        }
        _ => unreachable!(),
    };

    if actual_size != expected_size {
        return Err(invalid_data(format!(
            "uncompressed size mismatch for '{}'",
            entry.name
        )));
    }
    if actual_crc != entry.crc32 {
        return Err(invalid_data(format!("CRC32 mismatch for '{}'", entry.name)));
    }
    Ok(())
}

fn copy_and_checksum<R: Read, W: Write>(
    input: &mut R,
    output: &mut W,
    expected_size: u64,
) -> io::Result<(u64, u32)> {
    let mut checksum = crc32fast::Hasher::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 32 * 1024];

    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or_else(|| invalid_data("uncompressed data size overflows"))?;
        if total > expected_size {
            return Err(invalid_data("uncompressed data exceeds its declared size"));
        }
        checksum.update(&buffer[..count]);
        output.write_all(&buffer[..count])?;
    }

    Ok((total, checksum.finalize()))
}

fn ensure_extraction_root(root: &Path) -> io::Result<()> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(invalid_input(format!(
            "extraction directory '{}' is a symbolic link",
            root.display()
        ))),
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(invalid_input(format!(
            "extraction destination '{}' is not a directory",
            root.display()
        ))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(root)?;
            let metadata = fs::symlink_metadata(root)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(invalid_input("unsafe extraction directory"));
            }
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn ensure_safe_directory_tree(root: &Path, directory: &Path) -> io::Result<()> {
    let relative = directory
        .strip_prefix(root)
        .map_err(|_| invalid_data("ZIP entry escapes the extraction directory"))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(invalid_data(format!(
                    "refusing to follow symbolic link '{}'",
                    current.display()
                )));
            }
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("{} is not a directory", current.display()),
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(&current)?;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn reject_unsafe_existing_target(path: &Path, overwrite: bool) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(invalid_data(format!(
            "refusing to overwrite symbolic link '{}'",
            path.display()
        ))),
        Ok(metadata) if metadata.is_dir() => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is a directory", path.display()),
        )),
        Ok(_) if !overwrite => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists; use -o to overwrite", path.display()),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn safe_relative_path(name: &str) -> io::Result<PathBuf> {
    if name.is_empty() || name.starts_with('/') || name.starts_with('\\') {
        return Err(invalid_data(format!("unsafe ZIP entry path '{}'", name)));
    }

    let mut path = PathBuf::new();
    for component in name.split(['/', '\\']) {
        match component {
            "" | "." => {}
            ".." => {
                return Err(invalid_data(format!("unsafe ZIP entry path '{}'", name)));
            }
            _ if component.contains('\0') || component.contains(':') => {
                return Err(invalid_data(format!("unsafe ZIP entry path '{}'", name)));
            }
            _ => path.push(component),
        }
    }
    Ok(path)
}

fn decode_name(bytes: &[u8], flags: u16) -> io::Result<String> {
    if flags & (1 << 11) != 0 {
        return std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| invalid_data("ZIP entry has an invalid UTF-8 file name"));
    }
    if let Ok(name) = std::str::from_utf8(bytes) {
        return Ok(name.to_owned());
    }

    let mut name = String::with_capacity(bytes.len());
    for &byte in bytes {
        if byte < 0x80 {
            name.push(char::from(byte));
        } else {
            name.push(CP437[usize::from(byte - 0x80)]);
        }
    }
    Ok(name)
}

const CP437: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', 'É', 'æ', 'Æ',
    'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ', 'á', 'í', 'ó', 'ú', 'ñ', 'Ñ',
    'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»', '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕',
    '╣', '║', '╗', '╝', '╜', '╛', '┐', '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦',
    '╠', '═', '╬', '╧', '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐',
    '▀', 'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩', '≡', '±',
    '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', ' ',
];

fn le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

#[cfg(test)]
mod tests {
    use super::entry_unix_mode;
    #[cfg(unix)]
    use super::{create_output_file, run_unzip, Options};
    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::path::{Path, PathBuf};
    #[cfg(unix)]
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    #[cfg(unix)]
    struct TestDirectory(PathBuf);

    #[cfg(unix)]
    impl TestDirectory {
        fn new(label: &str) -> Self {
            for _ in 0..128 {
                let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "idlebox-unzip-{label}-{}-{sequence}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("failed to create test directory: {error}"),
                }
            }
            panic!("failed to choose a unique test directory");
        }
    }

    #[cfg(unix)]
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(unix)]
    struct StoredMember<'a> {
        name: &'a str,
        data: &'a [u8],
        mode: u32,
        crc_override: Option<u32>,
    }

    #[cfg(unix)]
    fn stored_member<'a>(name: &'a str, data: &'a [u8], mode: u32) -> StoredMember<'a> {
        StoredMember {
            name,
            data,
            mode,
            crc_override: None,
        }
    }

    #[cfg(unix)]
    fn write_u16(output: &mut Vec<u8>, value: u16) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    #[cfg(unix)]
    fn write_u32(output: &mut Vec<u8>, value: u32) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    #[cfg(unix)]
    fn write_stored_zip(path: &Path, members: &[StoredMember<'_>]) {
        let mut archive = Vec::new();
        let mut central_entries = Vec::new();

        for member in members {
            let name = member.name.as_bytes();
            let size = u32::try_from(member.data.len()).unwrap();
            let offset = u32::try_from(archive.len()).unwrap();
            let crc = member
                .crc_override
                .unwrap_or_else(|| crc32fast::hash(member.data));

            write_u32(&mut archive, 0x0403_4b50);
            write_u16(&mut archive, 20);
            write_u16(&mut archive, 0);
            write_u16(&mut archive, 0);
            write_u16(&mut archive, 0);
            write_u16(&mut archive, 0);
            write_u32(&mut archive, crc);
            write_u32(&mut archive, size);
            write_u32(&mut archive, size);
            write_u16(&mut archive, u16::try_from(name.len()).unwrap());
            write_u16(&mut archive, 0);
            archive.extend_from_slice(name);
            archive.extend_from_slice(member.data);

            let mut central = Vec::new();
            write_u32(&mut central, 0x0201_4b50);
            write_u16(&mut central, (3 << 8) | 20);
            write_u16(&mut central, 20);
            write_u16(&mut central, 0);
            write_u16(&mut central, 0);
            write_u16(&mut central, 0);
            write_u16(&mut central, 0);
            write_u32(&mut central, crc);
            write_u32(&mut central, size);
            write_u32(&mut central, size);
            write_u16(&mut central, u16::try_from(name.len()).unwrap());
            write_u16(&mut central, 0);
            write_u16(&mut central, 0);
            write_u16(&mut central, 0);
            write_u16(&mut central, 0);
            write_u32(&mut central, member.mode << 16);
            write_u32(&mut central, offset);
            central.extend_from_slice(name);
            central_entries.push(central);
        }

        let central_offset = u32::try_from(archive.len()).unwrap();
        for entry in central_entries {
            archive.extend_from_slice(&entry);
        }
        let central_size = u32::try_from(archive.len()).unwrap() - central_offset;
        write_u32(&mut archive, 0x0605_4b50);
        write_u16(&mut archive, 0);
        write_u16(&mut archive, 0);
        write_u16(&mut archive, u16::try_from(members.len()).unwrap());
        write_u16(&mut archive, u16::try_from(members.len()).unwrap());
        write_u32(&mut archive, central_size);
        write_u32(&mut archive, central_offset);
        write_u16(&mut archive, 0);

        fs::write(path, archive).unwrap();
    }

    #[cfg(unix)]
    fn extract(archive: &Path, destination: &Path, overwrite: bool) -> std::io::Result<()> {
        run_unzip(&Options {
            archive: archive.to_path_buf(),
            destination: destination.to_path_buf(),
            list: false,
            overwrite,
        })
    }

    #[test]
    fn unix_mode_distinguishes_missing_attributes_from_explicit_mode_zero() {
        let unix_made_by = (3 << 8) | 20;
        assert_eq!(entry_unix_mode(unix_made_by, 0), None);
        assert_eq!(entry_unix_mode(unix_made_by, 0o100000_u32 << 16), Some(0));
        assert_eq!(
            entry_unix_mode(unix_made_by, 0o100755_u32 << 16),
            Some(0o755)
        );
        assert_eq!(entry_unix_mode(20, 0o100755_u32 << 16), None);
    }

    #[cfg(unix)]
    #[test]
    fn file_with_archived_mode_starts_private() {
        use std::os::unix::fs::PermissionsExt;

        let test_directory = TestDirectory::new("private-staging");
        let output = test_directory.0.join("output");
        let file = create_output_file(&output, Some(0o755)).unwrap();

        assert_eq!(file.metadata().unwrap().permissions().mode() & 0o077, 0);
    }

    #[cfg(unix)]
    #[test]
    fn extraction_preserves_file_and_directory_modes() {
        use std::os::unix::fs::PermissionsExt;

        let test_directory = TestDirectory::new("permissions");
        let archive = test_directory.0.join("archive.zip");
        let destination = test_directory.0.join("output");
        write_stored_zip(
            &archive,
            &[
                stored_member("locked/", b"", 0o040555),
                stored_member("locked/tool", b"executable", 0o100755),
                stored_member("locked/private/", b"", 0o040500),
                stored_member("locked/private/secret", b"private", 0o100600),
            ],
        );

        extract(&archive, &destination, false).unwrap();

        assert_eq!(
            fs::read(destination.join("locked/tool")).unwrap(),
            b"executable"
        );
        assert_eq!(
            fs::read(destination.join("locked/private/secret")).unwrap(),
            b"private"
        );
        assert_eq!(
            fs::metadata(destination.join("locked"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o555
        );
        assert_eq!(
            fs::metadata(destination.join("locked/private"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o500
        );
        assert_eq!(
            fs::metadata(destination.join("locked/tool"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o755
        );
        assert_eq!(
            fs::metadata(destination.join("locked/private/secret"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o600
        );

        fs::set_permissions(
            destination.join("locked/private"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        fs::set_permissions(
            destination.join("locked"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn failed_entry_leaves_directory_modes_unapplied() {
        use std::os::unix::fs::PermissionsExt;

        let test_directory = TestDirectory::new("failed-directory-mode");
        let archive = test_directory.0.join("archive.zip");
        let destination = test_directory.0.join("output");
        let locked = destination.join("locked");
        fs::create_dir_all(&locked).unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
        write_stored_zip(
            &archive,
            &[
                stored_member("locked/", b"", 0o040500),
                StoredMember {
                    name: "locked/bad",
                    data: b"corrupt",
                    mode: 0o100755,
                    crc_override: Some(0),
                },
            ],
        );

        assert!(extract(&archive, &destination, false).is_err());
        assert_eq!(
            fs::metadata(&locked).unwrap().permissions().mode() & 0o7777,
            0o700
        );
        assert!(!locked.join("bad").exists());
    }

    #[cfg(unix)]
    #[test]
    fn crc_failure_preserves_existing_target_and_its_mode() {
        use std::os::unix::fs::PermissionsExt;

        let test_directory = TestDirectory::new("failed-overwrite");
        let archive = test_directory.0.join("archive.zip");
        let destination = test_directory.0.join("output");
        let target = destination.join("tool");
        fs::create_dir(&destination).unwrap();
        fs::write(&target, b"original").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
        write_stored_zip(
            &archive,
            &[StoredMember {
                name: "tool",
                data: b"replacement",
                mode: 0o100755,
                crc_override: Some(0),
            }],
        );

        assert!(extract(&archive, &destination, true).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"original");
        assert_eq!(
            fs::metadata(&target).unwrap().permissions().mode() & 0o7777,
            0o600
        );
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 1);
    }
}
