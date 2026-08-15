use crate::core::{file_ops::replace_file, Applet};
use flate2::read::MultiGzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct GzipApplet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GzipInvocation {
    Gzip,
    Gunzip,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operation {
    Compress,
    Decompress,
}

#[derive(Debug)]
struct Options<'a> {
    operation: Operation,
    keep: bool,
    force: bool,
    to_stdout: bool,
    files: Vec<&'a str>,
}

#[derive(Debug)]
enum ProcessError {
    Io { context: String, source: io::Error },
    Message(String),
}

impl ProcessError {
    fn io(context: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }

    fn is_broken_pipe(&self) -> bool {
        matches!(
            self,
            Self::Io { source, .. } if source.kind() == io::ErrorKind::BrokenPipe
        )
    }
}

impl fmt::Display for ProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { context, source } => write!(formatter, "{}: {}", context, source),
            Self::Message(message) => formatter.write_str(message),
        }
    }
}

impl Error for ProcessError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Message(_) => None,
        }
    }
}

impl Applet for GzipApplet {
    fn name(&self) -> &'static str {
        "gzip"
    }

    fn description(&self) -> &'static str {
        "Compress or decompress files using gzip"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn Error>> {
        run_gzip(args, GzipInvocation::Gzip)
    }

    fn help(&self) {
        println!("Usage: gzip [OPTION]... [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -d, --decompress  Decompress input");
        println!("  -k, --keep        Keep input files");
        println!("  -f, --force       Overwrite output files");
        println!("  -c, --to-stdout   Write to standard output");
        println!();
        println!("With no FILE, or when FILE is -, read standard input and write standard output.");
    }
}

pub(crate) fn run_gzip(args: &[String], invocation: GzipInvocation) -> Result<i32, Box<dyn Error>> {
    let program = match invocation {
        GzipInvocation::Gzip => "gzip",
        GzipInvocation::Gunzip => "gunzip",
    };
    let options = match parse_options(args, invocation) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{}: {}", program, message);
            return Ok(1);
        }
    };

    if options.files.is_empty() {
        return match transform_standard_stream(options.operation, program) {
            Ok(()) => Ok(0),
            Err(error) => {
                if error.is_broken_pipe() {
                    Err(Box::new(error))
                } else {
                    eprintln!("{}", error);
                    Ok(1)
                }
            }
        };
    }

    let mut failed = false;
    for input_name in &options.files {
        let result = if *input_name == "-" {
            transform_standard_stream(options.operation, program)
        } else if options.to_stdout {
            transform_file_to_stdout(Path::new(input_name), options.operation)
        } else {
            transform_file(
                Path::new(input_name),
                options.operation,
                options.keep,
                options.force,
            )
        };

        match result {
            Ok(()) => {}
            Err(error) if error.is_broken_pipe() => return Err(Box::new(error)),
            Err(error) => {
                eprintln!("{}: {}", program, error);
                failed = true;
            }
        }
    }

    Ok(i32::from(failed))
}

pub(crate) fn run_zcat(args: &[String]) -> Result<i32, Box<dyn Error>> {
    let files = match parse_zcat_options(args) {
        Ok(files) => files,
        Err(message) => {
            eprintln!("zcat: {}", message);
            return Ok(1);
        }
    };

    if files.is_empty() {
        return match transform_standard_stream(Operation::Decompress, "zcat") {
            Ok(()) => Ok(0),
            Err(error) => {
                if error.is_broken_pipe() {
                    Err(Box::new(error))
                } else {
                    eprintln!("{}", error);
                    Ok(1)
                }
            }
        };
    }

    let mut failed = false;
    for input_name in files {
        let result = if input_name == "-" {
            transform_standard_stream(Operation::Decompress, "zcat")
        } else {
            transform_file_to_stdout(Path::new(input_name), Operation::Decompress)
        };
        match result {
            Ok(()) => {}
            Err(error) if error.is_broken_pipe() => return Err(Box::new(error)),
            Err(error) => {
                eprintln!("zcat: {}", error);
                failed = true;
            }
        }
    }
    Ok(i32::from(failed))
}

fn parse_options(args: &[String], invocation: GzipInvocation) -> Result<Options<'_>, String> {
    let mut options = Options {
        operation: match invocation {
            GzipInvocation::Gzip => Operation::Compress,
            GzipInvocation::Gunzip => Operation::Decompress,
        },
        keep: false,
        force: false,
        to_stdout: false,
        files: Vec::new(),
    };
    let mut options_ended = false;

    for argument in args {
        if !options_ended {
            match argument.as_str() {
                "--" => {
                    options_ended = true;
                    continue;
                }
                "--decompress" => {
                    options.operation = Operation::Decompress;
                    continue;
                }
                "--keep" => {
                    options.keep = true;
                    continue;
                }
                "--force" => {
                    options.force = true;
                    continue;
                }
                "--stdout" | "--to-stdout" => {
                    options.to_stdout = true;
                    continue;
                }
                option if option.starts_with("--") => {
                    return Err(format!("unrecognized option '{}'", option));
                }
                option if option.starts_with('-') && option != "-" => {
                    for flag in option[1..].chars() {
                        match flag {
                            'd' => options.operation = Operation::Decompress,
                            'k' => options.keep = true,
                            'f' => options.force = true,
                            'c' => options.to_stdout = true,
                            _ => return Err(format!("invalid option -- '{}'", flag)),
                        }
                    }
                    continue;
                }
                _ => {}
            }
        }
        options.files.push(argument);
    }

    Ok(options)
}

fn parse_zcat_options(args: &[String]) -> Result<Vec<&str>, String> {
    let mut files = Vec::new();
    let mut options_ended = false;
    for argument in args {
        if !options_ended {
            match argument.as_str() {
                "--" => {
                    options_ended = true;
                    continue;
                }
                option if option.starts_with('-') && option != "-" => {
                    return Err(format!("invalid option -- '{}'", option));
                }
                _ => {}
            }
        }
        files.push(argument.as_str());
    }
    Ok(files)
}

fn transform_standard_stream(operation: Operation, program: &str) -> Result<(), ProcessError> {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        return Err(ProcessError::Message(format!(
            "{}: compressed data not read from a terminal. Use -f to force decompression.",
            program
        )));
    }
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    transform(&mut input, &mut output, operation)
        .map_err(|error| ProcessError::io("standard input", error))?;
    output
        .flush()
        .map_err(|error| ProcessError::io("standard output", error))
}

fn transform_file_to_stdout(input_path: &Path, operation: Operation) -> Result<(), ProcessError> {
    let mut input = File::open(input_path)
        .map_err(|error| ProcessError::io(display_path(input_path), error))?;
    let stdout = io::stdout();
    let mut output = stdout.lock();
    transform(&mut input, &mut output, operation)
        .map_err(|error| ProcessError::io(display_path(input_path), error))?;
    output
        .flush()
        .map_err(|error| ProcessError::io("standard output", error))
}

fn transform_file(
    input_path: &Path,
    operation: Operation,
    keep: bool,
    force: bool,
) -> Result<(), ProcessError> {
    let output_path = output_path(input_path, operation)?;
    if !force && path_entry_exists(&output_path)? {
        return Err(ProcessError::Message(format!(
            "{} already exists; use -f to overwrite it",
            display_path(&output_path)
        )));
    }

    let mut input = File::open(input_path)
        .map_err(|error| ProcessError::io(display_path(input_path), error))?;
    let input_metadata = input
        .metadata()
        .map_err(|error| ProcessError::io(display_path(input_path), error))?;
    let source_permissions = input_metadata
        .file_type()
        .is_file()
        .then(|| input_metadata.permissions());
    let mut temporary = TemporaryOutput::create(&output_path)?;
    transform(&mut input, temporary.writer(), operation)
        .map_err(|error| ProcessError::io(display_path(input_path), error))?;
    temporary.finish(&output_path, force, source_permissions)?;

    if !keep {
        fs::remove_file(input_path)
            .map_err(|error| ProcessError::io(display_path(input_path), error))?;
    }
    Ok(())
}

fn transform<R: Read, W: Write>(
    input: &mut R,
    output: &mut W,
    operation: Operation,
) -> io::Result<()> {
    match operation {
        Operation::Compress => {
            let mut encoder = GzEncoder::new(output, Compression::default());
            io::copy(input, &mut encoder)?;
            encoder.finish()?.flush()
        }
        Operation::Decompress => {
            let mut decoder = MultiGzDecoder::new(input);
            io::copy(&mut decoder, output)?;
            output.flush()
        }
    }
}

fn output_path(input_path: &Path, operation: Operation) -> Result<PathBuf, ProcessError> {
    match operation {
        Operation::Compress => {
            let mut name = input_path.as_os_str().to_os_string();
            name.push(".gz");
            Ok(PathBuf::from(name))
        }
        Operation::Decompress => decompressed_path(input_path),
    }
}

fn decompressed_path(input_path: &Path) -> Result<PathBuf, ProcessError> {
    let Some(file_name) = input_path.file_name().and_then(|name| name.to_str()) else {
        return Err(ProcessError::Message(format!(
            "{}: cannot determine output name",
            display_path(input_path)
        )));
    };

    let output_name = if let Some(stem) = file_name.strip_suffix(".gz") {
        if stem.is_empty() {
            return Err(ProcessError::Message(format!(
                "{}: empty output name",
                display_path(input_path)
            )));
        }
        OsString::from(stem)
    } else if let Some(stem) = file_name.strip_suffix(".tgz") {
        let mut name = OsString::from(stem);
        name.push(".tar");
        name
    } else {
        return Err(ProcessError::Message(format!(
            "{}: unknown suffix -- ignored",
            display_path(input_path)
        )));
    };

    Ok(input_path.with_file_name(output_name))
}

fn path_entry_exists(path: &Path) -> Result<bool, ProcessError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(ProcessError::io(display_path(path), error)),
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

static TEMPORARY_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TemporaryOutput {
    path: PathBuf,
    file: Option<File>,
    committed: bool,
}

impl TemporaryOutput {
    fn create(destination: &Path) -> Result<Self, ProcessError> {
        let parent = destination.parent().unwrap_or_else(|| Path::new("."));
        let file_name = destination
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("output"));

        for _ in 0..128 {
            let sequence = TEMPORARY_COUNTER.fetch_add(1, Ordering::Relaxed);
            let mut temporary_name = OsString::from(".");
            temporary_name.push(file_name);
            temporary_name.push(format!(".idlebox.{}.{}.tmp", std::process::id(), sequence));
            let path = parent.join(temporary_name);
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        path,
                        file: Some(file),
                        committed: false,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(ProcessError::io(display_path(&path), error)),
            }
        }

        Err(ProcessError::Message(format!(
            "{}: could not create a temporary output file",
            display_path(destination)
        )))
    }

    fn writer(&mut self) -> &mut File {
        self.file
            .as_mut()
            .expect("temporary output is open while it is written")
    }

    fn finish(
        mut self,
        destination: &Path,
        force: bool,
        source_permissions: Option<fs::Permissions>,
    ) -> Result<(), ProcessError> {
        if let Some(mut file) = self.file.take() {
            file.flush()
                .map_err(|error| ProcessError::io(display_path(&self.path), error))?;
            if let Some(permissions) = source_permissions {
                file.set_permissions(permissions)
                    .map_err(|error| ProcessError::io(display_path(&self.path), error))?;
            }
        }

        if force {
            let warning = replace_file(&self.path, destination)
                .map_err(|error| ProcessError::io(display_path(destination), error))?;
            if let Some(warning) = warning {
                eprintln!(
                    "gzip: warning: wrote '{}', but old backup '{}' could not be removed: {}",
                    destination.display(),
                    warning.backup_path.display(),
                    warning.error
                );
            }
        } else {
            fs::hard_link(&self.path, destination)
                .map_err(|error| ProcessError::io(display_path(destination), error))?;
            fs::remove_file(&self.path)
                .map_err(|error| ProcessError::io(display_path(&self.path), error))?;
        }

        self.committed = true;
        Ok(())
    }
}

impl Drop for TemporaryOutput {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decompressed_path, parse_options, transform_file, GzipInvocation, Operation};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIRECTORY_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn create() -> Self {
            for _ in 0..128 {
                let sequence = TEST_DIRECTORY_COUNTER.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "idlebox-gzip-test-{}-{}",
                    std::process::id(),
                    sequence
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

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn parses_combined_options_and_operands() {
        let args = ["-dkfc".to_owned(), "archive.gz".to_owned()];
        let options = parse_options(&args, GzipInvocation::Gzip).unwrap();
        assert_eq!(options.operation, Operation::Decompress);
        assert!(options.keep);
        assert!(options.force);
        assert!(options.to_stdout);
        assert_eq!(options.files, ["archive.gz"]);
    }

    #[test]
    fn maps_gzip_and_tgz_names() {
        assert_eq!(
            decompressed_path(Path::new("dir/archive.gz")).unwrap(),
            PathBuf::from("dir/archive")
        );
        assert_eq!(
            decompressed_path(Path::new("dir/archive.tgz")).unwrap(),
            PathBuf::from("dir/archive.tar")
        );
    }

    #[test]
    fn rejects_unknown_decompression_suffix() {
        assert!(decompressed_path(Path::new("archive.data")).is_err());
    }

    #[test]
    fn failed_forced_transform_preserves_input_and_destination() {
        let directory = TestDirectory::create();
        let input = directory.0.join("archive.gz");
        let output = directory.0.join("archive");
        fs::write(&input, b"not a gzip stream").unwrap();
        fs::write(&output, b"existing output").unwrap();

        assert!(transform_file(&input, Operation::Decompress, false, true).is_err());
        assert_eq!(fs::read(&input).unwrap(), b"not a gzip stream");
        assert_eq!(fs::read(&output).unwrap(), b"existing output");
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn file_transforms_preserve_private_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::create();
        let input = directory.0.join("secret");
        let compressed = directory.0.join("secret.gz");
        fs::write(&input, b"private data").unwrap();
        fs::set_permissions(&input, fs::Permissions::from_mode(0o600)).unwrap();

        transform_file(&input, Operation::Compress, false, false).unwrap();
        assert!(!input.exists());
        assert_eq!(
            fs::metadata(&compressed).unwrap().permissions().mode() & 0o7777,
            0o600
        );

        transform_file(&compressed, Operation::Decompress, false, false).unwrap();
        assert!(!compressed.exists());
        assert_eq!(fs::read(&input).unwrap(), b"private data");
        assert_eq!(
            fs::metadata(&input).unwrap().permissions().mode() & 0o7777,
            0o600
        );
    }
}
