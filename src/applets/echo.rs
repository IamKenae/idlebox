use crate::core::Applet;
use std::io::{self, Write};

pub struct EchoApplet;

impl Applet for EchoApplet {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn description(&self) -> &'static str {
        "Print text to standard output"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut newline = true;
        let mut start_idx = 0;

        if !args.is_empty() && args[0] == "-n" {
            newline = false;
            start_idx = 1;
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();

        for (index, arg) in args[start_idx..].iter().enumerate() {
            if index > 0 {
                out.write_all(b" ")?;
            }
            out.write_all(arg.as_bytes())?;
        }

        if newline {
            out.write_all(b"\n")?;
        }
        out.flush()?;

        Ok(0)
    }
}
