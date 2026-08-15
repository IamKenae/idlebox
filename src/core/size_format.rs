const UNITS: [&str; 7] = ["B", "K", "M", "G", "T", "P", "E"];

pub(crate) fn human_size(bytes: u64, include_byte_suffix: bool, include_tenths: bool) -> String {
    let mut divisor = 1u64;
    let mut unit = 0usize;

    while bytes / divisor >= 1024 && unit < UNITS.len() - 1 {
        divisor *= 1024;
        unit += 1;
    }

    if unit == 0 {
        return if include_byte_suffix {
            format!("{}B", bytes)
        } else {
            bytes.to_string()
        };
    }

    let divisor = u128::from(divisor);
    if include_tenths {
        let scaled = divide_round_ties_even(u128::from(bytes) * 10, divisor);
        format!("{}.{}{}", scaled / 10, scaled % 10, UNITS[unit])
    } else {
        let scaled = divide_round_ties_even(u128::from(bytes), divisor);
        format!("{}{}", scaled, UNITS[unit])
    }
}

#[cfg(any(target_os = "linux", windows, test))]
pub(crate) fn rounded_percentage(part: u64, total: u64) -> u64 {
    if total == 0 {
        return 0;
    }

    divide_round_ties_even(u128::from(part) * 100, u128::from(total)) as u64
}

fn divide_round_ties_even(numerator: u128, denominator: u128) -> u128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let doubled = remainder * 2;
    if doubled > denominator || (doubled == denominator && quotient % 2 == 1) {
        quotient + 1
    } else {
        quotient
    }
}

#[cfg(test)]
mod tests {
    use super::{human_size, rounded_percentage};

    #[test]
    fn formats_binary_sizes_without_floating_point() {
        assert_eq!(human_size(0, true, true), "0B");
        assert_eq!(human_size(1023, false, true), "1023");
        assert_eq!(human_size(1024, true, true), "1.0K");
        assert_eq!(human_size(1280, true, true), "1.2K");
        assert_eq!(human_size(1536, true, true), "1.5K");
        assert_eq!(human_size(1536, true, false), "2K");
        assert_eq!(human_size(2560, true, false), "2K");
        assert_eq!(human_size(u64::MAX, true, true), "16.0E");
    }

    #[test]
    fn rounds_percentages_without_overflow() {
        assert_eq!(rounded_percentage(0, 0), 0);
        assert_eq!(rounded_percentage(1, 3), 33);
        assert_eq!(rounded_percentage(2, 3), 67);
        assert_eq!(rounded_percentage(u64::MAX, u64::MAX), 100);
    }
}
