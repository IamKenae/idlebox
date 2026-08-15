pub trait Applet {
    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str;

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>>;

    fn help(&self) {
        println!("Usage: {} [OPTIONS]", self.name());
        println!("{}", self.description());
    }
}
