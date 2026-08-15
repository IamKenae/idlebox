use crate::core::{banner, Applet};
use std::io::{self, Write};

pub struct PrintfApplet;

impl Applet for PrintfApplet {
    fn name(&self) -> &'static str {
        "printf"
    }

    fn description(&self) -> &'static str {
        "Format and print arguments"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut index = 0;
        if args.first().map(String::as_str) == Some("--") {
            index = 1;
        }
        if index == args.len() {
            self.print_usage();
            return Ok(1);
        }

        let result = format_all(&args[index], &args[index + 1..]);
        let stdout = io::stdout();
        let mut out = stdout.lock();
        out.write_all(&result.output)?;
        out.flush()?;
        for diagnostic in result.diagnostics {
            eprintln!("printf: {}", diagnostic);
        }
        Ok(result.status)
    }

    fn help(&self) {
        println!("{}", banner());
        println!();
        println!("Usage: printf FORMAT [ARGUMENT]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Supports %s, %b, %c, signed and unsigned integer formats,");
        println!("floating-point formats, field width, precision, and backslash escapes.");
        println!("FORMAT is reused as necessary to consume all arguments.");
    }
}

impl PrintfApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: printf FORMAT [ARGUMENT]...");
        eprintln!();
        eprintln!("Supports %s, %b, %c, signed and unsigned integer formats,");
        eprintln!("floating-point formats, field width, precision, and backslash escapes.");
        eprintln!("FORMAT is reused as necessary to consume all arguments.");
    }
}

struct PrintfResult {
    output: Vec<u8>,
    diagnostics: Vec<String>,
    status: i32,
}

#[derive(Clone, Copy, Default)]
struct FormatOptions {
    alternate: bool,
    left: bool,
    plus: bool,
    space: bool,
    zero: bool,
    width: Option<usize>,
    precision: Option<usize>,
}

fn format_all(format: &str, arguments: &[String]) -> PrintfResult {
    let mut result = PrintfResult {
        output: Vec::new(),
        diagnostics: Vec::new(),
        status: 0,
    };
    let mut argument_index = 0;

    loop {
        let before = argument_index;
        match format_once(format, arguments, &mut argument_index, &mut result) {
            Ok(true) => break,
            Ok(false) => {}
            Err(error) => {
                result.diagnostics.push(error);
                result.status = 1;
                break;
            }
        }

        if argument_index >= arguments.len() || argument_index == before {
            break;
        }
    }

    result
}

fn format_once(
    format: &str,
    arguments: &[String],
    argument_index: &mut usize,
    result: &mut PrintfResult,
) -> Result<bool, String> {
    let bytes = format.as_bytes();
    let mut position = 0;

    while position < bytes.len() {
        match bytes[position] {
            b'\\' => {
                let (next, stop) = append_escape(bytes, position, false, &mut result.output);
                position = next;
                if stop {
                    return Ok(true);
                }
            }
            b'%' => {
                position += 1;
                if position == bytes.len() {
                    return Err("incomplete conversion specification".to_string());
                }
                if bytes[position] == b'%' {
                    result.output.push(b'%');
                    position += 1;
                    continue;
                }

                let mut options = FormatOptions::default();
                while position < bytes.len() {
                    match bytes[position] {
                        b'#' => options.alternate = true,
                        b'-' => options.left = true,
                        b'+' => options.plus = true,
                        b' ' => options.space = true,
                        b'0' => options.zero = true,
                        _ => break,
                    }
                    position += 1;
                }

                if position < bytes.len() && bytes[position] == b'*' {
                    let width = next_argument(arguments, argument_index);
                    let parsed = parse_width(width).map_err(|_| {
                        format!("invalid field width: '{}'", width.escape_default())
                    })?;
                    if parsed < 0 {
                        options.left = true;
                        options.width = usize::try_from(parsed.unsigned_abs()).ok();
                    } else {
                        options.width = usize::try_from(parsed).ok();
                    }
                    position += 1;
                } else {
                    let start = position;
                    while position < bytes.len() && bytes[position].is_ascii_digit() {
                        position += 1;
                    }
                    if position > start {
                        options.width = Some(parse_decimal(&bytes[start..position])?);
                    }
                }

                if position < bytes.len() && bytes[position] == b'.' {
                    position += 1;
                    if position < bytes.len() && bytes[position] == b'*' {
                        let precision = next_argument(arguments, argument_index);
                        let parsed = parse_width(precision).map_err(|_| {
                            format!("invalid precision: '{}'", precision.escape_default())
                        })?;
                        if parsed >= 0 {
                            options.precision = usize::try_from(parsed).ok();
                        }
                        position += 1;
                    } else {
                        let start = position;
                        while position < bytes.len() && bytes[position].is_ascii_digit() {
                            position += 1;
                        }
                        options.precision = Some(if start == position {
                            0
                        } else {
                            parse_decimal(&bytes[start..position])?
                        });
                    }
                }

                while position < bytes.len()
                    && matches!(bytes[position], b'h' | b'l' | b'L' | b'j' | b'z' | b't')
                {
                    position += 1;
                }
                if position == bytes.len() {
                    return Err("incomplete conversion specification".to_string());
                }

                let conversion = bytes[position];
                position += 1;
                let argument = if conversion == b'%' {
                    ""
                } else {
                    next_argument(arguments, argument_index)
                };

                match conversion {
                    b's' => append_bytes(
                        &mut result.output,
                        truncate(argument.as_bytes(), options.precision),
                        options,
                    ),
                    b'b' => {
                        let mut decoded = Vec::new();
                        let mut source_position = 0;
                        let source = argument.as_bytes();
                        let mut stop = false;
                        while source_position < source.len() {
                            if source[source_position] == b'\\' {
                                let (next, should_stop) =
                                    append_escape(source, source_position, true, &mut decoded);
                                source_position = next;
                                if should_stop {
                                    stop = true;
                                    break;
                                }
                            } else {
                                decoded.push(source[source_position]);
                                source_position += 1;
                            }
                        }
                        append_bytes(
                            &mut result.output,
                            truncate(&decoded, options.precision),
                            options,
                        );
                        if stop {
                            return Ok(true);
                        }
                    }
                    b'c' => {
                        let character = argument
                            .chars()
                            .next()
                            .map(char_bytes)
                            .unwrap_or_else(|| vec![0]);
                        append_bytes(&mut result.output, &character, options);
                    }
                    b'd' | b'i' => {
                        let value = parse_signed(argument, result);
                        append_integer(
                            &mut result.output,
                            value.unsigned_abs(),
                            value.is_negative(),
                            10,
                            false,
                            true,
                            options,
                        );
                    }
                    b'u' | b'o' | b'x' | b'X' => {
                        let value = parse_signed(argument, result) as u64 as u128;
                        let radix = match conversion {
                            b'o' => 8,
                            b'x' | b'X' => 16,
                            _ => 10,
                        };
                        append_integer(
                            &mut result.output,
                            value,
                            false,
                            radix,
                            conversion == b'X',
                            false,
                            options,
                        );
                    }
                    b'f' | b'F' | b'e' | b'E' | b'g' | b'G' => {
                        let value = parse_float(argument, result);
                        let rendered = render_float(value, conversion, options);
                        append_numeric_text(&mut result.output, rendered.as_bytes(), options);
                    }
                    _ => {
                        return Err(format!(
                            "unsupported conversion specification '%{}'",
                            char::from(conversion)
                        ));
                    }
                }
            }
            byte => {
                result.output.push(byte);
                position += 1;
            }
        }
    }

    Ok(false)
}

fn next_argument<'a>(arguments: &'a [String], index: &mut usize) -> &'a str {
    if let Some(argument) = arguments.get(*index) {
        *index += 1;
        argument
    } else {
        ""
    }
}

fn parse_decimal(bytes: &[u8]) -> Result<usize, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "invalid number".to_string())?;
    text.parse::<usize>()
        .map_err(|_| format!("field width is too large: '{}'", text))
}

fn parse_width(value: &str) -> Result<i64, ()> {
    if value.is_empty() {
        Ok(0)
    } else {
        value.parse().map_err(|_| ())
    }
}

fn parse_signed(argument: &str, result: &mut PrintfResult) -> i128 {
    if argument.is_empty() {
        return 0;
    }
    if let Some(character) = quoted_character(argument) {
        return i128::from(character);
    }

    let (negative, unsigned) = match argument.as_bytes().first() {
        Some(b'-') => (true, &argument[1..]),
        Some(b'+') => (false, &argument[1..]),
        _ => (false, argument),
    };
    let parsed = if let Some(hex) = unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))
    {
        i128::from_str_radix(hex, 16)
    } else if unsigned.len() > 1 && unsigned.starts_with('0') {
        i128::from_str_radix(&unsigned[1..], 8)
    } else {
        unsigned.parse::<i128>()
    };

    match parsed {
        Ok(value) => {
            if negative {
                -value
            } else {
                value
            }
        }
        Err(_) => {
            numeric_diagnostic(argument, result);
            0
        }
    }
}

fn parse_float(argument: &str, result: &mut PrintfResult) -> f64 {
    if argument.is_empty() {
        return 0.0;
    }
    if let Some(character) = quoted_character(argument) {
        return f64::from(character);
    }
    match argument.parse::<f64>() {
        Ok(value) => value,
        Err(_) => {
            numeric_diagnostic(argument, result);
            0.0
        }
    }
}

fn quoted_character(argument: &str) -> Option<u32> {
    match argument.as_bytes().first() {
        Some(b'\'') | Some(b'"') => argument[1..].chars().next().map(u32::from),
        _ => None,
    }
}

fn numeric_diagnostic(argument: &str, result: &mut PrintfResult) {
    result.diagnostics.push(format!(
        "'{}': expected a numeric value",
        argument.escape_default()
    ));
    result.status = 1;
}

fn render_float(value: f64, conversion: u8, options: FormatOptions) -> String {
    let precision = options.precision.unwrap_or(6);
    let mut rendered = match conversion {
        b'e' => normalize_exponent(format!("{value:.precision$e}"), false),
        b'E' => normalize_exponent(format!("{value:.precision$E}"), true),
        b'g' | b'G' => {
            let precision = precision.max(1);
            let uppercase = conversion == b'G';
            let scientific = format!("{value:.digits$e}", digits = precision - 1);
            let exponent = scientific_exponent(&scientific);
            let use_scientific =
                exponent.is_some_and(|value| value < -4 || value >= precision as i32);
            let mut rendered = if use_scientific {
                normalize_exponent(scientific, uppercase)
            } else if let Some(exponent) = exponent {
                let decimals = (precision as i32 - exponent - 1).max(0) as usize;
                format!("{value:.decimals$}")
            } else {
                value.to_string()
            };
            if !options.alternate {
                trim_float_zeros(&mut rendered);
            }
            if uppercase {
                rendered.make_ascii_uppercase();
            }
            rendered
        }
        _ => format!("{value:.precision$}"),
    };

    if value.is_sign_positive() && !value.is_nan() {
        if options.plus {
            rendered.insert(0, '+');
        } else if options.space {
            rendered.insert(0, ' ');
        }
    }
    if options.alternate && !rendered.contains('.') {
        if let Some(exponent) = rendered.find(['e', 'E']) {
            rendered.insert(exponent, '.');
        } else {
            rendered.push('.');
        }
    }
    rendered
}

fn scientific_exponent(value: &str) -> Option<i32> {
    let marker = value.find(['e', 'E'])?;
    value[marker + 1..].parse().ok()
}

fn normalize_exponent(value: String, uppercase: bool) -> String {
    let Some(marker) = value.find(['e', 'E']) else {
        return value;
    };
    let Some(exponent) = value[marker + 1..].parse::<i32>().ok() else {
        return value;
    };

    let mut normalized = String::from(&value[..marker]);
    normalized.push(if uppercase { 'E' } else { 'e' });
    normalized.push(if exponent < 0 { '-' } else { '+' });
    let magnitude = exponent.unsigned_abs();
    if magnitude < 10 {
        normalized.push('0');
    }
    normalized.push_str(&magnitude.to_string());
    normalized
}

fn trim_float_zeros(value: &mut String) {
    let exponent = value.find(['e', 'E']).unwrap_or(value.len());
    let Some(decimal) = value[..exponent].find('.') else {
        return;
    };
    let mut end = exponent;
    while end > decimal + 1 && value.as_bytes()[end - 1] == b'0' {
        end -= 1;
    }
    if end == decimal + 1 {
        end = decimal;
    }
    value.replace_range(end..exponent, "");
}

fn append_integer(
    output: &mut Vec<u8>,
    value: u128,
    negative: bool,
    radix: u8,
    uppercase: bool,
    signed: bool,
    options: FormatOptions,
) {
    let mut digits = unsigned_digits(value, radix, uppercase);
    if options.precision == Some(0) && value == 0 {
        digits.clear();
    }
    if let Some(precision) = options.precision {
        if digits.len() < precision {
            let mut padded = vec![b'0'; precision - digits.len()];
            padded.extend_from_slice(&digits);
            digits = padded;
        }
    }

    let mut prefix = Vec::new();
    if signed {
        if negative {
            prefix.push(b'-');
        } else if options.plus {
            prefix.push(b'+');
        } else if options.space {
            prefix.push(b' ');
        }
    }
    if options.alternate {
        match radix {
            8 if !digits.starts_with(b"0") => prefix.push(b'0'),
            16 if value != 0 => {
                prefix.push(b'0');
                prefix.push(if uppercase { b'X' } else { b'x' });
            }
            _ => {}
        }
    }

    let width = options.width.unwrap_or(0);
    let padding = width.saturating_sub(prefix.len() + digits.len());
    if !(options.left || options.zero && options.precision.is_none()) {
        output.extend(std::iter::repeat_n(b' ', padding));
    }
    output.extend_from_slice(&prefix);
    if !options.left && options.zero && options.precision.is_none() {
        output.extend(std::iter::repeat_n(b'0', padding));
    }
    output.extend_from_slice(&digits);
    if options.left {
        output.extend(std::iter::repeat_n(b' ', padding));
    }
}

fn append_numeric_text(output: &mut Vec<u8>, value: &[u8], options: FormatOptions) {
    let width = options.width.unwrap_or(0);
    let padding = width.saturating_sub(value.len());
    if options.left {
        output.extend_from_slice(value);
        output.extend(std::iter::repeat_n(b' ', padding));
    } else if options.zero && padding > 0 && matches!(value.first(), Some(b'+' | b'-' | b' ')) {
        output.push(value[0]);
        output.extend(std::iter::repeat_n(b'0', padding));
        output.extend_from_slice(&value[1..]);
    } else {
        output.extend(std::iter::repeat_n(
            if options.zero { b'0' } else { b' ' },
            padding,
        ));
        output.extend_from_slice(value);
    }
}

fn append_bytes(output: &mut Vec<u8>, value: &[u8], options: FormatOptions) {
    let width = options.width.unwrap_or(0);
    let padding = width.saturating_sub(value.len());
    if !options.left {
        output.extend(std::iter::repeat_n(b' ', padding));
    }
    output.extend_from_slice(value);
    if options.left {
        output.extend(std::iter::repeat_n(b' ', padding));
    }
}

fn truncate(value: &[u8], precision: Option<usize>) -> &[u8] {
    &value[..precision.unwrap_or(value.len()).min(value.len())]
}

fn char_bytes(character: char) -> Vec<u8> {
    let mut buffer = [0_u8; 4];
    character.encode_utf8(&mut buffer).as_bytes().to_vec()
}

fn unsigned_digits(mut value: u128, radix: u8, uppercase: bool) -> Vec<u8> {
    if value == 0 {
        return vec![b'0'];
    }
    let mut reversed = Vec::new();
    while value > 0 {
        let digit = (value % u128::from(radix)) as u8;
        reversed.push(match digit {
            0..=9 => b'0' + digit,
            _ if uppercase => b'A' + digit - 10,
            _ => b'a' + digit - 10,
        });
        value /= u128::from(radix);
    }
    reversed.reverse();
    reversed
}

fn append_escape(
    source: &[u8],
    slash: usize,
    plain_octal: bool,
    output: &mut Vec<u8>,
) -> (usize, bool) {
    if slash + 1 == source.len() {
        output.push(b'\\');
        return (source.len(), false);
    }

    let escaped = source[slash + 1];
    match escaped {
        b'a' => output.push(7),
        b'b' => output.push(8),
        b'c' => return (slash + 2, true),
        b'e' | b'E' => output.push(27),
        b'f' => output.push(12),
        b'n' => output.push(b'\n'),
        b'r' => output.push(b'\r'),
        b't' => output.push(b'\t'),
        b'v' => output.push(11),
        b'\\' => output.push(b'\\'),
        b'x' => {
            let (value, consumed) = parse_digits(&source[slash + 2..], 16, 2);
            if consumed == 0 {
                output.extend_from_slice(b"\\x");
                return (slash + 2, false);
            }
            output.push(value);
            return (slash + 2 + consumed, false);
        }
        b'0' if plain_octal => {
            let (value, consumed) = parse_digits(&source[slash + 2..], 8, 3);
            output.push(value);
            return (slash + 2 + consumed, false);
        }
        b'1'..=b'7' if plain_octal => {
            let (value, consumed) = parse_digits(&source[slash + 1..], 8, 3);
            output.push(value);
            return (slash + 1 + consumed, false);
        }
        b'0'..=b'7' => {
            let (value, consumed) = parse_digits(&source[slash + 1..], 8, 3);
            output.push(value);
            return (slash + 1 + consumed, false);
        }
        _ => {
            output.push(b'\\');
            output.push(escaped);
        }
    }
    (slash + 2, false)
}

fn parse_digits(source: &[u8], radix: u8, limit: usize) -> (u8, usize) {
    let mut value = 0_u16;
    let mut consumed = 0;
    for byte in source.iter().copied().take(limit) {
        let Some(digit) = digit_value(byte) else {
            break;
        };
        if digit >= radix {
            break;
        }
        value = value * u16::from(radix) + u16::from(digit);
        consumed += 1;
    }
    (value as u8, consumed)
}

fn digit_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::format_all;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn repeats_format_to_consume_arguments() {
        let result = format_all("[%s]", &strings(&["one", "two"]));
        assert_eq!(result.output, b"[one][two]");
        assert_eq!(result.status, 0);
    }

    #[test]
    fn formats_integers_width_and_precision() {
        let result = format_all("%+05d %#x %.3s", &strings(&["7", "31", "hello"]));
        assert_eq!(result.output, b"+0007 0x1f hel");
        assert_eq!(result.status, 0);
    }

    #[test]
    fn expands_percent_b_escapes() {
        let result = format_all("%b", &strings(&["a\\tb\\n"]));
        assert_eq!(result.output, b"a\tb\n");
        assert_eq!(result.status, 0);
    }

    #[test]
    fn percent_b_c_stops_all_output() {
        let result = format_all("before:%b:after", &strings(&["one\\ctwo"]));
        assert_eq!(result.output, b"before:one");
    }

    #[test]
    fn formats_scientific_and_general_floats() {
        let result = format_all("%.2e|%.3g", &strings(&["12.5", "12345"]));
        assert_eq!(result.output, b"1.25e+01|1.23e+04");
    }

    #[test]
    fn format_octal_escape_uses_three_digits() {
        let result = format_all("\\101|\\0101", &[]);
        assert_eq!(result.output, [b'A', b'|', 8, b'1']);
    }

    #[test]
    fn missing_character_argument_defaults_to_nul() {
        let result = format_all("%cX", &[]);
        assert_eq!(result.output, [0, b'X']);
    }
}
