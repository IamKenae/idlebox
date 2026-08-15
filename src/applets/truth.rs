use crate::core::Applet;

pub struct TrueApplet;

impl Applet for TrueApplet {
    fn name(&self) -> &'static str {
        "true"
    }

    fn description(&self) -> &'static str {
        "Return a successful exit status"
    }

    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        Ok(0)
    }

    fn help(&self) {
        println!("Usage: true");
        println!();
        println!("Return a successful exit status.");
    }
}

pub struct FalseApplet;

impl Applet for FalseApplet {
    fn name(&self) -> &'static str {
        "false"
    }

    fn description(&self) -> &'static str {
        "Return an unsuccessful exit status"
    }

    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        Ok(1)
    }

    fn help(&self) {
        println!("Usage: false");
        println!();
        println!("Return an unsuccessful exit status.");
    }
}
