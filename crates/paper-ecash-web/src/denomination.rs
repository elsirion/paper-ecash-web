/// Minimum denomination exponent (2^10 = 1024 msat)
pub const MIN_POWER: u32 = 10;
/// Maximum denomination exponent (2^34 ≈ 17.2 Gsat)
pub const MAX_POWER: u32 = 34;
/// Max denominations per paper note
pub const MAX_SELECTIONS: usize = 4;

/// All available fedimint denominations as msat values (powers of 2).
pub fn available_denominations() -> Vec<u64> {
    (MIN_POWER..=MAX_POWER).map(|p| 1u64 << p).collect()
}

/// Format msat amount using SI prefixes, matching note-selector style.
pub fn format_amount_msat(msat: u64) -> String {
    const PREFIXES: &[(u64, &str)] = &[
        (1_000_000_000_000_000, "Tsat"),
        (1_000_000_000_000, "Gsat"),
        (1_000_000_000, "Msat"),
        (1_000_000, "ksat"),
        (1_000, "sat"),
        (1, "msat"),
    ];

    if msat == 0 {
        return "0 msat".to_string();
    }

    for &(multiplier, symbol) in PREFIXES {
        if msat >= multiplier {
            let value = msat as f64 / multiplier as f64;
            return format!("{} {symbol}", format_sig_figs(value, 3));
        }
    }

    format!("{msat} msat")
}

/// Format a float to N significant figures, trimming trailing zeros.
fn format_sig_figs(value: f64, sig_figs: usize) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let digits = value.log10().floor() as i32 + 1;
    let decimals = (sig_figs as i32 - digits).max(0) as usize;
    let s = format!("{value:.decimals$}");
    // Trim trailing zeros after decimal point
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_denominations() {
        let denoms = available_denominations();
        assert_eq!(denoms.len(), 25);
        assert_eq!(denoms[0], 1024);
        assert_eq!(denoms[24], 1u64 << 34);
    }

    #[test]
    fn test_format_amount() {
        assert_eq!(format_amount_msat(1024), "1.02 ksat");
        assert_eq!(format_amount_msat(1_048_576), "1.05 Msat");
        assert_eq!(format_amount_msat(1_073_741_824), "1.07 Gsat");
        assert_eq!(format_amount_msat(0), "0 msat");
        assert_eq!(format_amount_msat(500), "500 msat");
        assert_eq!(format_amount_msat(1000), "1 sat");
    }

    #[test]
    fn test_format_sig_figs() {
        assert_eq!(format_sig_figs(1.024, 3), "1.02");
        assert_eq!(format_sig_figs(1048.576, 3), "1050");
        assert_eq!(format_sig_figs(100.0, 3), "100");
    }
}
