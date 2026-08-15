use crate::core::{
    banner,
    file_ops::{replace_file, same_file, unique_sibling_path, FollowSymlinks},
    Applet,
};
use flate2::read::MultiGzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

const BLOCK_SIZE: usize = 512;
const ZERO_BLOCK: [u8; BLOCK_SIZE] = [0; BLOCK_SIZE];

pub struct TarApplet;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Create,
    Extract,
    List,
}

struct Options {
    mode: Mode,
    archive: String,
    gzip: bool,
    verbose: bool,
    directory: Option<PathBuf>,
    operands: Vec<String>,
}

impl Applet for TarApplet {
    fn name(&self) -> &'static str {
        "tar"
    }

    fn description(&self) -> &'static str {
        "Create, extract, or list tar archives"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let options = match parse_options(args) {
            Ok(options) => options,
            Err(message) => {
                eprintln!("{}", banner());
                eprintln!();
                eprintln!("tar: {message}");
                eprintln!();
                eprintln!("Usage: tar (-c|-x|-t) [-zv] [-f ARCHIVE] [-C DIR] [FILE]...");
                return Ok(1);
            }
        };

        match options.mode {
            Mode::Create => create(&options)?,
            Mode::Extract | Mode::List => read_archive(&options)?,
        }
        Ok(0)
    }

    fn help(&self) {
        println!("{}", banner());
        println!();
        println!("Usage: tar (-c|-x|-t) [-zv] [-f ARCHIVE] [-C DIR] [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -c, --create          Create an archive");
        println!("  -x, --extract         Extract an archive");
        println!("  -t, --list            List archive contents");
        println!("  -f, --file ARCHIVE    Use ARCHIVE ('-' means standard input/output)");
        println!("  -z, --gzip            Filter the archive through gzip");
        println!("  -C, --directory DIR   Change the input or extraction directory");
        println!("  -v, --verbose         List files as they are processed");
    }
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut mode = None;
    let mut archive = None;
    let mut gzip = false;
    let mut verbose = false;
    let mut directory = None;
    let mut operands = Vec::new();
    let mut options_ended = false;
    let mut index = 0;

    while index < args.len() {
        let argument = &args[index];
        if !options_ended && argument == "--" {
            options_ended = true;
            index += 1;
            continue;
        }

        if !options_ended && argument.starts_with("--") {
            let (name, attached) = argument
                .split_once('=')
                .map_or((argument.as_str(), None), |(name, value)| {
                    (name, Some(value))
                });
            match name {
                "--create" => set_mode(&mut mode, Mode::Create)?,
                "--extract" => set_mode(&mut mode, Mode::Extract)?,
                "--list" => set_mode(&mut mode, Mode::List)?,
                "--gzip" => gzip = true,
                "--verbose" => verbose = true,
                "--file" => {
                    archive = Some(option_value(args, &mut index, attached, "--file")?);
                }
                "--directory" => {
                    directory = Some(PathBuf::from(option_value(
                        args,
                        &mut index,
                        attached,
                        "--directory",
                    )?));
                }
                _ => return Err(format!("unrecognized option '{argument}'")),
            }
            index += 1;
            continue;
        }

        if !options_ended && argument.starts_with('-') && argument != "-" {
            let bytes = argument.as_bytes();
            let mut position = 1;
            while position < bytes.len() {
                match bytes[position] {
                    b'c' => set_mode(&mut mode, Mode::Create)?,
                    b'x' => set_mode(&mut mode, Mode::Extract)?,
                    b't' => set_mode(&mut mode, Mode::List)?,
                    b'z' => gzip = true,
                    b'v' => verbose = true,
                    b'f' | b'C' => {
                        let flag = bytes[position];
                        let value = if position + 1 < bytes.len() {
                            argument[position + 1..].to_owned()
                        } else {
                            index += 1;
                            args.get(index).cloned().ok_or_else(|| {
                                format!("option requires an argument -- '{}'", flag as char)
                            })?
                        };
                        if flag == b'f' {
                            archive = Some(value);
                        } else {
                            directory = Some(PathBuf::from(value));
                        }
                        position = bytes.len();
                        continue;
                    }
                    flag => {
                        return Err(format!("invalid option -- '{}'", flag as char));
                    }
                }
                position += 1;
            }
            index += 1;
            continue;
        }

        operands.push(argument.clone());
        index += 1;
    }

    let mode = mode.ok_or_else(|| "one of -c, -x, or -t is required".to_owned())?;
    if mode == Mode::Create && operands.is_empty() {
        return Err("refusing to create an empty archive".to_owned());
    }

    Ok(Options {
        mode,
        archive: archive.unwrap_or_else(|| "-".to_owned()),
        gzip,
        verbose,
        directory,
        operands,
    })
}

fn set_mode(current: &mut Option<Mode>, requested: Mode) -> Result<(), String> {
    match current {
        Some(mode) if *mode != requested => {
            Err("options -c, -x, and -t are mutually exclusive".to_owned())
        }
        _ => {
            *current = Some(requested);
            Ok(())
        }
    }
}

fn option_value(
    args: &[String],
    index: &mut usize,
    attached: Option<&str>,
    name: &str,
) -> Result<String, String> {
    if let Some(value) = attached {
        if value.is_empty() {
            return Err(format!("option '{name}' requires an argument"));
        }
        return Ok(value.to_owned());
    }
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("option '{name}' requires an argument"))
}

fn create(options: &Options) -> io::Result<()> {
    let exclusion = prepare_create(options)?;
    let Some(archive) = exclusion else {
        let mut output = io::stdout();
        if options.gzip {
            let mut encoder = GzEncoder::new(output, Compression::default());
            write_archive(&mut encoder, options, &[])?;
            encoder.try_finish()?;
        } else {
            write_archive(&mut output, options, &[])?;
            output.flush()?;
        }
        return Ok(());
    };

    let (publish_target, existing_permissions) = prepare_archive_target(&archive)?;
    let staged = unique_sibling_path(&publish_target, "tar-create")?;
    let mut open_options = OpenOptions::new();
    open_options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        if let Some(permissions) = existing_permissions.as_ref() {
            open_options.mode(permissions.mode() & 0o777);
        }
    }
    let output = open_options
        .open(&staged)
        .map_err(|error| contextual_error(format!("{}: {error}", staged.display())))?;
    let result = (|| {
        let exclusions = [
            archive.as_path(),
            publish_target.as_path(),
            staged.as_path(),
        ];
        if options.gzip {
            let mut encoder = GzEncoder::new(output, Compression::default());
            write_archive(&mut encoder, options, &exclusions)?;
            let mut output = encoder.finish()?;
            output.flush()?;
        } else {
            let mut output = output;
            write_archive(&mut output, options, &exclusions)?;
            output.flush()?;
        }

        if let Some(permissions) = existing_permissions {
            fs::set_permissions(&staged, permissions)?;
        }

        if let Some(warning) = replace_file(&staged, &publish_target)? {
            eprintln!(
                "tar: warning: created '{}', but old backup '{}' could not be removed: {}",
                archive.display(),
                warning.backup_path.display(),
                warning.error
            );
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    result
}

fn prepare_archive_target(archive: &Path) -> io::Result<(PathBuf, Option<fs::Permissions>)> {
    let permissions = match fs::metadata(archive) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(contextual_error(format!(
                    "cannot create '{}': output is not a regular file",
                    archive.display()
                )));
            }
            OpenOptions::new()
                .write(true)
                .open(archive)
                .map_err(|error| {
                    contextual_error(format!("cannot create '{}': {error}", archive.display()))
                })?;
            Some(metadata.permissions())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    Ok((follow_final_symlinks(archive)?, permissions))
}

fn follow_final_symlinks(path: &Path) -> io::Result<PathBuf> {
    let mut target = path.to_path_buf();
    for _ in 0..40 {
        match fs::symlink_metadata(&target) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let link = fs::read_link(&target)?;
                target = if link.is_absolute() {
                    link
                } else {
                    nonempty_parent(&target).join(link)
                };
            }
            Ok(_) => return Ok(target),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(target),
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("tar: too many symbolic links in '{}'", path.display()),
    ))
}

fn prepare_create(options: &Options) -> io::Result<Option<PathBuf>> {
    if options.archive == "-" {
        return Ok(None);
    }

    let archive = PathBuf::from(&options.archive);
    let base = options
        .directory
        .as_deref()
        .unwrap_or_else(|| Path::new("."));
    let archive_has_target = match fs::metadata(&archive) {
        Ok(_) => true,
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(error),
    };

    // Check every explicit input before preparing the output. In particular,
    // this rejects an input named through another hard link or through the
    // archive path's symlink target.
    for operand in &options.operands {
        normalize_create_name(operand)?;
        let source = base.join(operand);
        preflight_create_path(&source, &archive, archive_has_target, false)?;
    }

    Ok(Some(archive))
}

fn preflight_create_path(
    source: &Path,
    archive: &Path,
    archive_has_target: bool,
    recursive_member: bool,
) -> io::Result<()> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| contextual_error(format!("{}: {error}", source.display())))?;
    let file_type = metadata.file_type();

    // Tar stores a symlink itself, so never follow it while inspecting inputs.
    // This also deliberately permits dangling links.
    if file_type.is_symlink() {
        if !recursive_member && same_directory_entry(source, archive)? {
            return Err(invalid_data(format!(
                "refusing to archive '{}' into itself",
                source.display()
            )));
        }
        return Ok(());
    }

    if file_type.is_file() {
        if !archive_has_target {
            return Ok(());
        }
        match same_file(source, archive, FollowSymlinks::Yes) {
            Ok(true) if recursive_member && same_directory_entry(source, archive)? => return Ok(()),
            Ok(true) => {
                return Err(invalid_data(format!(
                    "refusing to archive '{}' into itself",
                    source.display()
                )));
            }
            Ok(false) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        }
    }

    if file_type.is_dir() {
        // A missing output cannot share an identity with an input. Avoid walking
        // every new archive's tree twice; write_path will still validate it.
        if !archive_has_target {
            return Ok(());
        }
        for child in fs::read_dir(source)? {
            preflight_create_path(&child?.path(), archive, true, true)?;
        }
        return Ok(());
    }

    Err(invalid_data(format!(
        "{}: unsupported file type",
        source.display()
    )))
}

fn same_directory_entry(left: &Path, right: &Path) -> io::Result<bool> {
    let Some(left_name) = left.file_name() else {
        return Ok(false);
    };
    let Some(right_name) = right.file_name() else {
        return Ok(false);
    };
    if !file_names_equal(left_name, right_name) {
        return Ok(false);
    }

    let left_parent = nonempty_parent(left);
    let right_parent = nonempty_parent(right);
    same_file(left_parent, right_parent, FollowSymlinks::Yes)
}

fn nonempty_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

#[cfg(windows)]
fn file_names_equal(left: &std::ffi::OsStr, right: &std::ffi::OsStr) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(not(windows))]
fn file_names_equal(left: &std::ffi::OsStr, right: &std::ffi::OsStr) -> bool {
    left == right
}

fn write_archive(
    output: &mut dyn Write,
    options: &Options,
    exclusions: &[&Path],
) -> io::Result<()> {
    let base = match &options.directory {
        Some(directory) => directory.clone(),
        None => PathBuf::from("."),
    };

    for operand in &options.operands {
        let archive_name = normalize_create_name(operand)?;
        let source = base.join(operand);
        write_path(
            output,
            &source,
            &archive_name,
            options.verbose,
            options.archive == "-",
            exclusions,
        )?;
    }

    output.write_all(&ZERO_BLOCK)?;
    output.write_all(&ZERO_BLOCK)?;
    Ok(())
}

fn normalize_create_name(name: &str) -> io::Result<String> {
    if name == "." || name == "./" {
        return Ok(".".to_owned());
    }
    let path = safe_member_path(name)?;
    let normalized = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    if normalized.is_empty() {
        Err(invalid_data("empty archive member name"))
    } else {
        Ok(normalized)
    }
}

fn write_path(
    output: &mut dyn Write,
    source: &Path,
    archive_name: &str,
    verbose: bool,
    archive_is_stdout: bool,
    exclusions: &[&Path],
) -> io::Result<()> {
    for archive in exclusions {
        if source_is_archive(source, archive)? {
            return Ok(());
        }
    }

    let metadata = fs::symlink_metadata(source)
        .map_err(|error| contextual_error(format!("{}: {error}", source.display())))?;
    let file_type = metadata.file_type();
    let (typeflag, size, link_name, display_name) = if file_type.is_dir() {
        let name = format!("{}/", archive_name.trim_end_matches('/'));
        (b'5', 0, None, name)
    } else if file_type.is_file() {
        (b'0', metadata.len(), None, archive_name.to_owned())
    } else if file_type.is_symlink() {
        let target = fs::read_link(source)?;
        let target = target.to_string_lossy().replace('\\', "/");
        (b'2', 0, Some(target), archive_name.to_owned())
    } else {
        return Err(invalid_data(format!(
            "{}: unsupported file type",
            source.display()
        )));
    };

    let header = build_header(
        &display_name,
        &metadata,
        size,
        typeflag,
        link_name.as_deref(),
    )?;
    output.write_all(&header)?;
    if verbose {
        if archive_is_stdout {
            eprintln!("{display_name}");
        } else {
            println!("{display_name}");
        }
    }

    if file_type.is_file() {
        let mut file = File::open(source)?;
        let copied = io::copy(&mut file, output)?;
        if copied != size {
            return Err(invalid_data(format!(
                "{} changed size while being archived",
                source.display()
            )));
        }
        write_padding(output, size)?;
    } else if file_type.is_dir() {
        let mut children = fs::read_dir(source)?.collect::<Result<Vec<_>, _>>()?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            let child_name = child.file_name().to_string_lossy().into_owned();
            let child_archive_name = if archive_name == "." {
                child_name
            } else {
                format!("{}/{child_name}", archive_name.trim_end_matches('/'))
            };
            write_path(
                output,
                &child.path(),
                &child_archive_name,
                verbose,
                archive_is_stdout,
                exclusions,
            )?;
        }
    }
    Ok(())
}

fn source_is_archive(source: &Path, archive: &Path) -> io::Result<bool> {
    let metadata = match fs::symlink_metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    // A symlink encountered during recursion is data in its own right and does
    // not read its target. Exclude it only when it is the archive path itself.
    // Regular files are compared after following the archive path, which also
    // catches its actual target and every hard-link alias.
    let follow = if metadata.file_type().is_symlink() {
        FollowSymlinks::No
    } else {
        FollowSymlinks::Yes
    };
    match same_file(source, archive, follow) {
        Ok(same) => Ok(same),
        // Keep dangling symlinks as archive members, and let the normal metadata
        // path report missing inputs. Neither can identify an existing archive.
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn build_header(
    name: &str,
    metadata: &fs::Metadata,
    size: u64,
    typeflag: u8,
    link_name: Option<&str>,
) -> io::Result<[u8; BLOCK_SIZE]> {
    let mut header = [0_u8; BLOCK_SIZE];
    let (prefix, leaf) = split_ustar_name(name)?;
    copy_field(&mut header[0..100], leaf.as_bytes(), "file name")?;
    if let Some(prefix) = prefix {
        copy_field(&mut header[345..500], prefix.as_bytes(), "file name prefix")?;
    }

    write_octal(&mut header[100..108], metadata_mode(metadata))?;
    write_octal(&mut header[108..116], metadata_uid(metadata))?;
    write_octal(&mut header[116..124], metadata_gid(metadata))?;
    write_octal(&mut header[124..136], size)?;
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_secs());
    write_octal(&mut header[136..148], mtime)?;
    header[148..156].fill(b' ');
    header[156] = typeflag;
    if let Some(target) = link_name {
        copy_field(&mut header[157..257], target.as_bytes(), "link name")?;
    }
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");

    let checksum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
    let encoded = format!("{checksum:06o}\0 ");
    if encoded.len() != 8 {
        return Err(invalid_data("tar header checksum overflow"));
    }
    header[148..156].copy_from_slice(encoded.as_bytes());
    Ok(header)
}

fn split_ustar_name(name: &str) -> io::Result<(Option<&str>, &str)> {
    if name.len() <= 100 {
        return Ok((None, name));
    }
    for (position, _) in name.match_indices('/').rev() {
        let prefix = &name[..position];
        let leaf = &name[position + 1..];
        if !leaf.is_empty() && leaf.len() <= 100 && prefix.len() <= 155 {
            return Ok((Some(prefix), leaf));
        }
    }
    Err(invalid_data(format!(
        "archive member name is too long for ustar: {name}"
    )))
}

fn copy_field(destination: &mut [u8], value: &[u8], label: &str) -> io::Result<()> {
    if value.len() > destination.len() {
        return Err(invalid_data(format!("{label} is too long")));
    }
    destination[..value.len()].copy_from_slice(value);
    Ok(())
}

fn write_octal(destination: &mut [u8], value: u64) -> io::Result<()> {
    let digits = format!("{value:o}");
    let length = destination.len();
    if digits.len() >= length {
        return Err(invalid_data("numeric value does not fit in ustar header"));
    }
    destination.fill(b'0');
    let start = length - 1 - digits.len();
    destination[start..length - 1].copy_from_slice(digits.as_bytes());
    destination[length - 1] = 0;
    Ok(())
}

fn read_archive(options: &Options) -> io::Result<()> {
    let source: Box<dyn Read> = if options.archive == "-" {
        Box::new(io::stdin())
    } else {
        Box::new(File::open(&options.archive).map_err(|error| {
            contextual_error(format!("cannot open '{}': {error}", options.archive))
        })?)
    };
    let mut input: Box<dyn Read> = if options.gzip {
        Box::new(MultiGzDecoder::new(source))
    } else {
        source
    };

    let root = if options.mode == Mode::Extract {
        let directory = options
            .directory
            .as_deref()
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(directory)?;
        Some(
            fs::canonicalize(directory)
                .map_err(|error| contextual_error(format!("{}: {error}", directory.display())))?,
        )
    } else {
        None
    };
    let mut directory_modes = Vec::new();

    loop {
        let mut block = [0_u8; BLOCK_SIZE];
        if !read_block(&mut input, &mut block)? {
            break;
        }
        if block.iter().all(|byte| *byte == 0) {
            break;
        }

        let entry = parse_header(&block)?;
        let selected = member_selected(&entry.name, &options.operands);
        if selected && options.mode == Mode::List {
            println!("{}", entry.name);
        }

        if selected && options.mode == Mode::Extract {
            extract_entry(
                &mut input,
                root.as_deref().unwrap(),
                &entry,
                options.verbose,
                &mut directory_modes,
            )?;
        } else {
            skip_exact(&mut input, entry.size)?;
        }
        skip_padding(&mut input, entry.size)?;
    }
    if let Some(root) = root.as_deref() {
        apply_directory_modes(root, &mut directory_modes)?;
    }
    Ok(())
}

struct Entry {
    name: String,
    size: u64,
    mode: u32,
    typeflag: u8,
    link_name: String,
}

fn parse_header(header: &[u8; BLOCK_SIZE]) -> io::Result<Entry> {
    let expected = parse_octal(&header[148..156], "checksum")?;
    let actual: u64 = header
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            if (148..156).contains(&index) {
                u64::from(b' ')
            } else {
                u64::from(*byte)
            }
        })
        .sum();
    if expected != actual {
        return Err(invalid_data(format!(
            "invalid tar header checksum (expected {expected}, calculated {actual})"
        )));
    }

    let leaf = field_string(&header[0..100]);
    let prefix = field_string(&header[345..500]);
    let name = if prefix.is_empty() {
        leaf
    } else {
        format!("{prefix}/{leaf}")
    };
    if name.is_empty() {
        return Err(invalid_data("tar header has an empty member name"));
    }

    Ok(Entry {
        name,
        mode: parse_octal(&header[100..108], "mode")? as u32,
        size: parse_octal(&header[124..136], "size")?,
        typeflag: header[156],
        link_name: field_string(&header[157..257]),
    })
}

fn field_string(field: &[u8]) -> String {
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

fn parse_octal(field: &[u8], label: &str) -> io::Result<u64> {
    let value = field
        .iter()
        .copied()
        .skip_while(|byte| *byte == b' ' || *byte == 0)
        .take_while(|byte| *byte != b' ' && *byte != 0)
        .collect::<Vec<_>>();
    if value.is_empty() {
        return Ok(0);
    }
    if value.iter().any(|byte| !(b'0'..=b'7').contains(byte)) {
        return Err(invalid_data(format!("invalid octal {label} in tar header")));
    }
    let text = std::str::from_utf8(&value)
        .map_err(|_| invalid_data(format!("invalid {label} in tar header")))?;
    u64::from_str_radix(text, 8)
        .map_err(|_| invalid_data(format!("{label} overflows in tar header")))
}

fn extract_entry(
    input: &mut dyn Read,
    root: &Path,
    entry: &Entry,
    verbose: bool,
    directory_modes: &mut Vec<(PathBuf, u32)>,
) -> io::Result<()> {
    let relative = safe_member_path(&entry.name)?;
    if verbose {
        println!("{}", entry.name);
    }

    match entry.typeflag {
        b'\0' | b'0' | b'7' => {
            secure_parent_directories(root, &relative)?;
            let target = root.join(&relative);
            extract_regular_file(input, root, &relative, &target, entry)?;
        }
        b'5' => {
            secure_create_directory(root, &relative)?;
            skip_exact(input, entry.size)?;
            directory_modes.push((relative, entry.mode));
        }
        b'2' => {
            secure_parent_directories(root, &relative)?;
            validate_symlink_target(&relative, &entry.link_name)?;
            let target = root.join(&relative);
            if fs::symlink_metadata(&target).is_ok() {
                return Err(invalid_data(format!(
                    "refusing to replace existing path with symlink: {}",
                    target.display()
                )));
            }
            create_symlink(Path::new(&entry.link_name), &target)?;
            skip_exact(input, entry.size)?;
        }
        flag => {
            skip_exact(input, entry.size)?;
            return Err(invalid_data(format!(
                "unsupported tar entry type '{}' for '{}'",
                char::from(flag),
                entry.name
            )));
        }
    }
    Ok(())
}

fn extract_regular_file(
    input: &mut dyn Read,
    root: &Path,
    relative: &Path,
    target: &Path,
    entry: &Entry,
) -> io::Result<()> {
    reject_symlink(target)?;
    let staged = unique_sibling_path(target, "tar")?;
    let mut open_options = OpenOptions::new();
    open_options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open_options.mode(0o600);
    }
    let mut output = open_options
        .open(&staged)
        .map_err(|error| contextual_error(format!("{}: {error}", staged.display())))?;
    let result = (|| {
        let mut limited = input.take(entry.size);
        let copied = io::copy(&mut limited, &mut output)?;
        if copied != entry.size {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("truncated data for '{}'", entry.name),
            ));
        }
        output.flush()?;
        drop(output);
        set_permissions(&staged, entry.mode)?;

        // Revalidate the path after reading the entry so a late symlink cannot
        // make the staged file escape the extraction root or replace a link.
        secure_parent_directories(root, relative)?;
        reject_symlink(target)?;
        if let Some(warning) = replace_file(&staged, target)? {
            eprintln!(
                "tar: warning: extracted '{}', but old backup '{}' could not be removed: {}",
                target.display(),
                warning.backup_path.display(),
                warning.error
            );
        }
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    result
}

fn safe_member_path(name: &str) -> io::Result<PathBuf> {
    if name.is_empty()
        || name.contains('\\')
        || name.starts_with('/')
        || looks_like_windows_prefix(name)
    {
        return Err(invalid_data(format!("unsafe archive member path: {name}")));
    }

    let mut result = PathBuf::new();
    for component in Path::new(name).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => result.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(invalid_data(format!("unsafe archive member path: {name}")));
            }
        }
    }
    if result.as_os_str().is_empty() && name != "." && name != "./" {
        return Err(invalid_data(format!("unsafe archive member path: {name}")));
    }
    Ok(result)
}

fn looks_like_windows_prefix(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn secure_parent_directories(root: &Path, relative: &Path) -> io::Result<()> {
    let mut current = root.to_path_buf();
    if let Some(parent) = relative.parent() {
        for component in parent.components() {
            if let Component::Normal(part) = component {
                current.push(part);
                match fs::symlink_metadata(&current) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        return Err(invalid_data(format!(
                            "refusing to follow symlink while extracting: {}",
                            current.display()
                        )));
                    }
                    Ok(metadata) if !metadata.is_dir() => {
                        return Err(invalid_data(format!(
                            "not a directory: {}",
                            current.display()
                        )));
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        fs::create_dir(&current)?;
                    }
                    Err(error) => return Err(error),
                }
            }
        }
    }
    Ok(())
}

fn secure_create_directory(root: &Path, relative: &Path) -> io::Result<()> {
    secure_parent_directories(root, relative)?;
    let target = root.join(relative);
    match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(invalid_data(format!(
            "refusing to follow symlink while extracting: {}",
            target.display()
        ))),
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(invalid_data(format!(
            "not a directory: {}",
            target.display()
        ))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(&target),
        Err(error) => Err(error),
    }
}

fn apply_directory_modes(root: &Path, directories: &mut [(PathBuf, u32)]) -> io::Result<()> {
    // A stable sort preserves archive order for duplicate directory headers, so
    // the last header at a given depth still determines the final mode.
    directories.sort_by(|(left, _), (right, _)| {
        right.components().count().cmp(&left.components().count())
    });

    for (relative, mode) in directories {
        validate_extracted_directory(root, relative)?;
        let target = root.join(relative);
        set_permissions(&target, *mode).map_err(|error| {
            contextual_error(format!(
                "cannot set permissions on '{}': {error}",
                target.display()
            ))
        })?;
    }
    Ok(())
}

fn validate_extracted_directory(root: &Path, relative: &Path) -> io::Result<()> {
    validate_directory_component(root)?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        if let Component::Normal(part) = component {
            current.push(part);
            validate_directory_component(&current)?;
        }
    }
    Ok(())
}

fn validate_directory_component(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(invalid_data(format!(
            "refusing to follow symlink while setting directory permissions: {}",
            path.display()
        ))),
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(invalid_data(format!(
            "expected extracted directory: {}",
            path.display()
        ))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(invalid_data(format!(
            "extracted directory disappeared: {}",
            path.display()
        ))),
        Err(error) => Err(error),
    }
}

fn reject_symlink(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(invalid_data(format!(
            "refusing to overwrite symlink while extracting: {}",
            path.display()
        ))),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn validate_symlink_target(member: &Path, target: &str) -> io::Result<()> {
    if target.is_empty()
        || target.contains('\\')
        || Path::new(target).is_absolute()
        || looks_like_windows_prefix(target)
    {
        return Err(invalid_data(format!("unsafe symlink target: {target}")));
    }

    let mut depth = member.parent().map_or(0, |parent| {
        parent
            .components()
            .filter(|component| matches!(component, Component::Normal(_)))
            .count()
    });
    for component in Path::new(target).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => depth += 1,
            Component::ParentDir if depth > 0 => depth -= 1,
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(invalid_data(format!("unsafe symlink target: {target}")));
            }
        }
    }
    Ok(())
}

fn member_selected(name: &str, filters: &[String]) -> bool {
    if filters.is_empty() {
        return true;
    }
    let normalized = name.trim_start_matches("./").trim_end_matches('/');
    filters.iter().any(|filter| {
        let filter = filter.trim_start_matches("./").trim_end_matches('/');
        normalized == filter
            || normalized
                .strip_prefix(filter)
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}

fn read_block(input: &mut dyn Read, block: &mut [u8; BLOCK_SIZE]) -> io::Result<bool> {
    let mut offset = 0;
    while offset < block.len() {
        let count = input.read(&mut block[offset..])?;
        if count == 0 {
            if offset == 0 {
                return Ok(false);
            }
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "truncated tar header",
            ));
        }
        offset += count;
    }
    Ok(true)
}

fn skip_exact(input: &mut dyn Read, count: u64) -> io::Result<()> {
    let copied = io::copy(&mut input.take(count), &mut io::sink())?;
    if copied == count {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "truncated tar member data",
        ))
    }
}

fn write_padding(output: &mut dyn Write, size: u64) -> io::Result<()> {
    let padding = (BLOCK_SIZE as u64 - size % BLOCK_SIZE as u64) % BLOCK_SIZE as u64;
    output.write_all(&ZERO_BLOCK[..padding as usize])
}

fn skip_padding(input: &mut dyn Read, size: u64) -> io::Result<()> {
    let padding = (BLOCK_SIZE as u64 - size % BLOCK_SIZE as u64) % BLOCK_SIZE as u64;
    skip_exact(input, padding)
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("tar: {}", message.into()),
    )
}

fn contextual_error(message: impl Into<String>) -> io::Error {
    io::Error::other(format!("tar: {}", message.into()))
}

#[cfg(unix)]
fn metadata_mode(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    u64::from(metadata.mode() & 0o7777)
}

#[cfg(not(unix))]
fn metadata_mode(metadata: &fs::Metadata) -> u64 {
    let file_type = metadata.file_type();
    portable_metadata_mode(
        file_type.is_dir(),
        file_type.is_symlink(),
        metadata.permissions().readonly(),
    )
}

#[cfg(any(test, not(unix)))]
fn portable_metadata_mode(is_directory: bool, is_symlink: bool, readonly: bool) -> u64 {
    if is_symlink {
        0o777
    } else if is_directory {
        if readonly {
            0o555
        } else {
            0o755
        }
    } else if readonly {
        0o444
    } else {
        0o644
    }
}

#[cfg(unix)]
fn metadata_uid(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    u64::from(metadata.uid())
}

#[cfg(not(unix))]
fn metadata_uid(_metadata: &fs::Metadata) -> u64 {
    0
}

#[cfg(unix)]
fn metadata_gid(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    u64::from(metadata.gid())
}

#[cfg(not(unix))]
fn metadata_gid(_metadata: &fs::Metadata) -> u64 {
    0
}

#[cfg(unix)]
fn set_permissions(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o7777))
}

#[cfg(not(unix))]
fn set_permissions(_path: &Path, _mode: u32) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn create_symlink(_target: &Path, _link: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "tar: symbolic link extraction is not supported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::apply_directory_modes;
    use super::{
        create, extract_entry, parse_header, portable_metadata_mode, read_archive,
        safe_member_path, validate_symlink_target, Entry, Mode, Options, BLOCK_SIZE, ZERO_BLOCK,
    };
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::fs;
    use std::io::{Cursor, Write};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let number = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "idlebox-tar-{label}-{}-{number}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create test directory");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn create_options(directory: &Path, archive: &Path, operand: &str) -> Options {
        Options {
            mode: Mode::Create,
            archive: archive.to_string_lossy().into_owned(),
            gzip: false,
            verbose: false,
            directory: Some(directory.to_path_buf()),
            operands: vec![operand.to_owned()],
        }
    }

    #[test]
    fn unsafe_member_names_are_rejected() {
        assert!(safe_member_path("../escape").is_err());
        assert!(safe_member_path("/absolute").is_err());
        assert!(safe_member_path("C:/drive").is_err());
        assert!(safe_member_path("safe/path").is_ok());
    }

    #[test]
    fn escaping_symlink_targets_are_rejected() {
        assert!(validate_symlink_target(Path::new("link"), "../escape").is_err());
        assert!(validate_symlink_target(Path::new("dir/link"), "../inside").is_ok());
    }

    #[test]
    fn zero_header_is_not_a_member() {
        assert!(parse_header(&ZERO_BLOCK).is_err());
    }

    #[test]
    fn create_rejects_direct_input_without_truncating_it() {
        let directory = TestDirectory::new("create-direct");
        let archive = directory.0.join("archive.tar");
        fs::write(&archive, b"original data").unwrap();

        let error = create(&create_options(&directory.0, &archive, "archive.tar")).unwrap_err();

        assert!(error.to_string().contains("into itself"));
        assert_eq!(fs::read(&archive).unwrap(), b"original data");
    }

    #[test]
    fn create_rejects_hard_link_alias_without_truncating_it() {
        let directory = TestDirectory::new("create-hard-link");
        let source = directory.0.join("source");
        let archive = directory.0.join("archive.tar");
        fs::write(&source, b"original data").unwrap();
        fs::hard_link(&source, &archive).unwrap();

        let error = create(&create_options(&directory.0, &archive, "source")).unwrap_err();

        assert!(error.to_string().contains("into itself"));
        assert_eq!(fs::read(&source).unwrap(), b"original data");
        assert_eq!(fs::read(&archive).unwrap(), b"original data");
    }

    #[test]
    fn create_rejects_hard_link_alias_inside_recursive_input() {
        let directory = TestDirectory::new("create-recursive-hard-link");
        let input = directory.0.join("input");
        fs::create_dir(&input).unwrap();
        let source = input.join("data");
        let archive = directory.0.join("archive.tar");
        fs::write(&source, b"original data").unwrap();
        fs::hard_link(&source, &archive).unwrap();

        let error = create(&create_options(&directory.0, &archive, "input")).unwrap_err();

        assert!(error.to_string().contains("into itself"));
        assert_eq!(fs::read(&source).unwrap(), b"original data");
        assert_eq!(fs::read(&archive).unwrap(), b"original data");
    }

    #[cfg(unix)]
    #[test]
    fn create_rejects_symlink_alias_without_truncating_its_target() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new("create-symlink");
        let source = directory.0.join("source");
        let archive = directory.0.join("archive.tar");
        fs::write(&source, b"original data").unwrap();
        symlink(&source, &archive).unwrap();

        let error = create(&create_options(&directory.0, &archive, "source")).unwrap_err();

        assert!(error.to_string().contains("into itself"));
        assert_eq!(fs::read(&source).unwrap(), b"original data");
    }

    #[cfg(unix)]
    #[test]
    fn create_rejects_archive_symlink_target_inside_recursive_input() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new("create-recursive-symlink-target");
        let input = directory.0.join("input");
        fs::create_dir(&input).unwrap();
        let source = input.join("data");
        let archive = directory.0.join("archive.tar");
        fs::write(&source, b"original data").unwrap();
        symlink(&source, &archive).unwrap();

        let error = create(&create_options(&directory.0, &archive, "input")).unwrap_err();

        assert!(error.to_string().contains("into itself"));
        assert_eq!(fs::read(&source).unwrap(), b"original data");
        assert!(fs::symlink_metadata(&archive)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn create_excludes_archive_inside_recursive_input() {
        let directory = TestDirectory::new("create-recursive");
        fs::write(directory.0.join("payload"), b"payload").unwrap();
        let archive = directory.0.join("archive.tar");
        #[cfg(unix)]
        std::os::unix::fs::symlink("archive.tar", directory.0.join("archive-link")).unwrap();

        create(&create_options(&directory.0, &archive, ".")).unwrap();

        let bytes = fs::read(&archive).unwrap();
        let mut names = Vec::new();
        let mut offset = 0;
        while bytes[offset..offset + BLOCK_SIZE]
            .iter()
            .any(|byte| *byte != 0)
        {
            let header: &[u8; BLOCK_SIZE] = bytes[offset..offset + BLOCK_SIZE].try_into().unwrap();
            let entry = parse_header(header).unwrap();
            names.push(entry.name);
            offset += BLOCK_SIZE + entry.size.div_ceil(BLOCK_SIZE as u64) as usize * BLOCK_SIZE;
        }
        assert!(names.iter().any(|name| name == "./"));
        assert!(names.iter().any(|name| name == "payload"));
        assert!(!names.iter().any(|name| name == "archive.tar"));
        #[cfg(unix)]
        assert!(names.iter().any(|name| name == "archive-link"));
    }

    #[test]
    fn failed_create_preserves_existing_archive_and_cleans_staging_file() {
        let directory = TestDirectory::new("create-staged-failure");
        let source = directory.0.join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("x".repeat(101)), b"data").unwrap();
        let archive = directory.0.join("archive.tar");
        fs::write(&archive, b"original archive").unwrap();

        let error = create(&create_options(&source, &archive, ".")).unwrap_err();

        assert!(error.to_string().contains("too long for ustar"));
        assert_eq!(fs::read(&archive).unwrap(), b"original archive");
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn successful_create_preserves_existing_archive_mode() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new("create-preserve-mode");
        let source = directory.0.join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("payload"), b"data").unwrap();
        let archive = directory.0.join("archive.tar");
        fs::write(&archive, b"old archive").unwrap();
        fs::set_permissions(&archive, fs::Permissions::from_mode(0o600)).unwrap();

        create(&create_options(&source, &archive, "payload")).unwrap();

        assert_eq!(
            fs::metadata(&archive).unwrap().permissions().mode() & 0o7777,
            0o600
        );
        assert_ne!(fs::read(&archive).unwrap(), b"old archive");
    }

    #[cfg(unix)]
    #[test]
    fn readonly_existing_archive_is_not_replaced_without_write_access() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new("create-readonly");
        let source = directory.0.join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("payload"), b"data").unwrap();
        let archive = directory.0.join("archive.tar");
        fs::write(&archive, b"old archive").unwrap();
        fs::set_permissions(&archive, fs::Permissions::from_mode(0o400)).unwrap();
        if fs::OpenOptions::new().write(true).open(&archive).is_ok() {
            // A privileged test process may legitimately bypass the mode bits.
            return;
        }

        let error = create(&create_options(&source, &archive, "payload")).unwrap_err();

        assert!(error.to_string().contains("cannot create"));
        assert_eq!(fs::read(&archive).unwrap(), b"old archive");
        assert_eq!(
            fs::metadata(&archive).unwrap().permissions().mode() & 0o7777,
            0o400
        );
    }

    #[cfg(unix)]
    #[test]
    fn create_follows_output_symlink_and_preserves_the_link() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new("create-output-symlink");
        let source = directory.0.join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("payload"), b"data").unwrap();
        let target = directory.0.join("real.tar");
        let archive = directory.0.join("alias.tar");
        fs::write(&target, b"old archive").unwrap();
        symlink("real.tar", &archive).unwrap();

        create(&create_options(&source, &archive, "payload")).unwrap();

        assert!(fs::symlink_metadata(&archive)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_ne!(fs::read(&target).unwrap(), b"old archive");
        assert_eq!(fs::read(&archive).unwrap(), fs::read(&target).unwrap());
    }

    #[test]
    fn truncated_entry_does_not_damage_existing_target() {
        let directory = TestDirectory::new("extract-truncated");
        let target = directory.0.join("target");
        fs::write(&target, b"original data").unwrap();
        let root = fs::canonicalize(&directory.0).unwrap();
        let entry = Entry {
            name: "target".to_owned(),
            size: 8,
            mode: 0o644,
            typeflag: b'0',
            link_name: String::new(),
        };
        let mut input = Cursor::new(b"short".to_vec());

        let error = extract_entry(&mut input, &root, &entry, false, &mut Vec::new()).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
        assert_eq!(fs::read(&target).unwrap(), b"original data");
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
    }

    #[test]
    fn portable_modes_do_not_make_regular_files_executable() {
        assert_eq!(portable_metadata_mode(false, false, false), 0o644);
        assert_eq!(portable_metadata_mode(false, false, true), 0o444);
        assert_eq!(portable_metadata_mode(true, false, false), 0o755);
        assert_eq!(portable_metadata_mode(true, false, true), 0o555);
        assert_eq!(portable_metadata_mode(false, true, false), 0o777);
        assert_eq!(portable_metadata_mode(false, true, true), 0o777);
    }

    #[test]
    fn gzip_mode_reads_concatenated_members_as_one_tar_stream() {
        let directory = TestDirectory::new("multi-member-gzip");
        let source = directory.0.join("source");
        let destination = directory.0.join("destination");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("payload"), b"multi-member data").unwrap();
        let plain_archive = directory.0.join("archive.tar");
        create(&create_options(&source, &plain_archive, "payload")).unwrap();
        let tar = fs::read(&plain_archive).unwrap();

        let mut gzip = Vec::new();
        for part in [&tar[..137], &tar[137..700], &tar[700..]] {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(part).unwrap();
            gzip.extend(encoder.finish().unwrap());
        }
        let gzip_archive = directory.0.join("archive.tar.gz");
        fs::write(&gzip_archive, gzip).unwrap();
        let options = Options {
            mode: Mode::Extract,
            archive: gzip_archive.to_string_lossy().into_owned(),
            gzip: true,
            verbose: false,
            directory: Some(destination.clone()),
            operands: Vec::new(),
        };

        read_archive(&options).unwrap();

        assert_eq!(
            fs::read(destination.join("payload")).unwrap(),
            b"multi-member data"
        );
    }

    #[cfg(unix)]
    #[test]
    fn directory_modes_are_applied_deepest_first_after_extraction() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new("extract-directory-modes");
        let parent = directory.0.join("parent");
        let child = parent.join("child");
        let root = fs::canonicalize(&directory.0).unwrap();
        let mut modes = Vec::new();
        let parent_entry = Entry {
            name: "parent".to_owned(),
            size: 0,
            mode: 0,
            typeflag: b'5',
            link_name: String::new(),
        };
        let child_entry = Entry {
            name: "parent/child".to_owned(),
            size: 0,
            mode: 0o500,
            typeflag: b'5',
            link_name: String::new(),
        };
        let file_entry = Entry {
            name: "parent/child/file".to_owned(),
            size: 4,
            mode: 0o600,
            typeflag: b'0',
            link_name: String::new(),
        };

        extract_entry(
            &mut Cursor::new(Vec::new()),
            &root,
            &parent_entry,
            false,
            &mut modes,
        )
        .unwrap();
        extract_entry(
            &mut Cursor::new(Vec::new()),
            &root,
            &child_entry,
            false,
            &mut modes,
        )
        .unwrap();
        extract_entry(
            &mut Cursor::new(b"data".to_vec()),
            &root,
            &file_entry,
            false,
            &mut modes,
        )
        .unwrap();
        assert_eq!(fs::read(child.join("file")).unwrap(), b"data");

        apply_directory_modes(&root, &mut modes).unwrap();

        assert_eq!(
            fs::metadata(&parent).unwrap().permissions().mode() & 0o7777,
            0
        );
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            fs::metadata(&child).unwrap().permissions().mode() & 0o7777,
            0o500
        );
        fs::set_permissions(&child, fs::Permissions::from_mode(0o700)).unwrap();
    }
}
