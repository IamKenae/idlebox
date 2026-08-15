use crate::core::Applet;
use std::thread;
use std::time::Duration;

pub struct RelaxApplet;

impl Applet for RelaxApplet {
    fn name(&self) -> &'static str {
        "relax"
    }

    fn description(&self) -> &'static str {
        "IdleBox special: take a break and relax"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let seconds = if !args.is_empty() {
            args[0].parse::<u64>().unwrap_or(5)
        } else {
            5
        };

        println!("Relaxing for {} seconds...", seconds);

        thread::sleep(Duration::from_secs(seconds));

        println!("Refreshed! Back to work.");
        Ok(0)
    }

    fn help(&self) {
        println!("Usage: relax [SECONDS]");
        println!();
        println!("{}", self.description());
        println!();
        println!("Arguments:");
        println!("  SECONDS    Duration to relax (default: 5)");
        println!();
        println!("Examples:");
        println!("  relax        # Relax for 5 seconds");
        println!("  relax 10     # Relax for 10 seconds");
    }
}
