use super::gzip::{run_gzip, GzipInvocation};
use crate::core::Applet;
use std::error::Error;

pub struct GunzipApplet;

impl Applet for GunzipApplet {
    fn name(&self) -> &'static str {
        "gunzip"
    }

    fn description(&self) -> &'static str {
        "Decompress gzip files"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn Error>> {
        run_gzip(args, GzipInvocation::Gunzip)
    }

    fn help(&self) {
        println!("Usage: gunzip [OPTION]... [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -k, --keep        Keep input files");
        println!("  -f, --force       Overwrite output files");
        println!("  -c, --to-stdout   Write to standard output");
        println!();
        println!("With no FILE, or when FILE is -, read standard input and write standard output.");
    }
}
