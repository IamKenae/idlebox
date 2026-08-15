use super::gzip::run_zcat;
use crate::core::Applet;
use std::error::Error;

pub struct ZcatApplet;

impl Applet for ZcatApplet {
    fn name(&self) -> &'static str {
        "zcat"
    }

    fn description(&self) -> &'static str {
        "Decompress gzip data to standard output"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn Error>> {
        run_zcat(args)
    }

    fn help(&self) {
        println!("Usage: zcat [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("With no FILE, or when FILE is -, read standard input.");
    }
}
