use crate::core::Applet;

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
        
        let output = args[start_idx..].join(" ");
        
        if newline {
            println!("{}", output);
        } else {
            print!("{}", output);
        }
        
        Ok(0)
    }
}
