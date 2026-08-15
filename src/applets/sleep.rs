use crate::core::Applet;
use std::thread;
use std::time::Duration;

pub struct SleepApplet;

impl Applet for SleepApplet {
    fn name(&self) -> &'static str {
        "sleep"
    }

    fn description(&self) -> &'static str {
        "Pause for a specified amount of time"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        if args.is_empty() {
            eprintln!("sleep: missing operand");
            return Ok(1);
        }

        let mut total = Duration::ZERO;
        for value in args {
            let duration = match parse_duration(value) {
                Some(duration) => duration,
                None => {
                    eprintln!("sleep: invalid time interval '{}'", value);
                    return Ok(1);
                }
            };
            let Some(sum) = total.checked_add(duration) else {
                eprintln!("sleep: time interval is too large");
                return Ok(1);
            };
            total = sum;
        }

        thread::sleep(total);
        Ok(0)
    }

    fn help(&self) {
        println!("Usage: sleep NUMBER[SUFFIX]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("SUFFIX may be s for seconds (default), m for minutes,");
        println!("h for hours, or d for days. NUMBER may be fractional.");
        println!("Multiple intervals are added together.");
    }
}

fn parse_duration(input: &str) -> Option<Duration> {
    let (number, multiplier) = match input.as_bytes().last().copied() {
        Some(b's') => (&input[..input.len() - 1], 1.0),
        Some(b'm') => (&input[..input.len() - 1], 60.0),
        Some(b'h') => (&input[..input.len() - 1], 3_600.0),
        Some(b'd') => (&input[..input.len() - 1], 86_400.0),
        Some(byte) if byte.is_ascii_alphabetic() => return None,
        Some(_) => (input, 1.0),
        None => return None,
    };

    if number.is_empty() {
        return None;
    }
    let seconds = number.parse::<f64>().ok()? * multiplier;
    if !seconds.is_finite() || seconds.is_sign_negative() {
        return None;
    }
    Duration::try_from_secs_f64(seconds).ok()
}

#[cfg(test)]
mod tests {
    use super::parse_duration;
    use std::time::Duration;

    #[test]
    fn parses_fractional_and_suffixed_durations() {
        assert_eq!(parse_duration("0.5"), Some(Duration::from_millis(500)));
        assert_eq!(parse_duration("2m"), Some(Duration::from_secs(120)));
        assert_eq!(parse_duration("1.5h"), Some(Duration::from_secs(5_400)));
        assert_eq!(parse_duration("1d"), Some(Duration::from_secs(86_400)));
    }

    #[test]
    fn rejects_invalid_durations() {
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("-1"), None);
        assert_eq!(parse_duration("1w"), None);
        assert_eq!(parse_duration("NaN"), None);
    }
}
