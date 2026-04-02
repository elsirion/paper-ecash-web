/// Decompose amount_msat into powers of 2 (fedimint denominations).
/// Returns None if more than `max_denominations` are needed.
pub fn decompose_amount(amount_msat: u64, max_denominations: usize) -> Option<Vec<u64>> {
    if amount_msat == 0 {
        return None;
    }
    let mut denoms = Vec::new();
    for bit in (0..64).rev() {
        if amount_msat & (1u64 << bit) != 0 {
            denoms.push(1u64 << bit);
        }
    }
    if denoms.len() > max_denominations {
        None
    } else {
        Some(denoms)
    }
}

/// Convert sats to msat and decompose.
pub fn decompose_sats(sats: u64, max_denominations: usize) -> Option<Vec<u64>> {
    decompose_amount(sats * 1000, max_denominations)
}

/// Find the nearest valid amounts (with <= max_denominations set bits) around target_sats.
/// Returns (lower, upper) options.
pub fn suggest_nearest_valid(target_sats: u64, max_denominations: usize) -> (Option<u64>, Option<u64>) {
    let target_msat = target_sats * 1000;
    // Search downward
    let lower = (1..=target_msat)
        .rev()
        .map(|m| m)
        .find(|&m| {
            let bits = m.count_ones() as usize;
            bits <= max_denominations && m % 1000 == 0
        });
    // Search upward (bounded)
    let upper_limit = target_msat.saturating_add(target_msat / 2);
    let upper = (target_msat + 1..=upper_limit).find(|&m| {
        let bits = m.count_ones() as usize;
        bits <= max_denominations && m % 1000 == 0
    });
    (lower.map(|m| m / 1000), upper.map(|m| m / 1000))
}

/// Format msat denomination as human-readable string.
pub fn format_denomination_msat(msat: u64) -> String {
    if msat >= 1_000_000_000 {
        let btc = msat as f64 / 100_000_000_000.0;
        format!("{btc:.8} BTC")
    } else if msat >= 1_000 {
        let sats = msat / 1000;
        format!("{sats} sat{}", if sats != 1 { "s" } else { "" })
    } else {
        format!("{msat} msat")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompose_powers_of_two() {
        // 1024 sats = 1_024_000 msat = 2^10 * 1000 = single denomination
        let result = decompose_sats(1024, 4);
        assert_eq!(result, Some(vec![1_024_000]));
    }

    #[test]
    fn test_decompose_multiple() {
        // 3 sats = 3000 msat = 2048 + 1024 = 2 denominations (in msat: bits of 3000)
        // 3000 = 0b101110111000 → bits: 2048, 512, 256, 128, 64, 8 → that's > 4
        // Actually fedimint denominations are powers of 2 msat
        // 3000 = 2048 + 512 + 256 + 128 + 32 + 16 + 8 → 7 bits set
        let result = decompose_amount(3000, 4);
        assert_eq!(result, None);
    }

    #[test]
    fn test_decompose_exact_powers() {
        // 1 sat = 1000 msat. 1000 in binary = 1111101000, has 6 bits set
        let result = decompose_amount(1000, 4);
        assert_eq!(result, None);

        // 1024 msat = single power of 2
        let result = decompose_amount(1024, 4);
        assert_eq!(result, Some(vec![1024]));

        // 3072 msat = 2048 + 1024 = 2 denominations
        let result = decompose_amount(3072, 4);
        assert_eq!(result, Some(vec![2048, 1024]));
    }

    #[test]
    fn test_format_denomination() {
        assert_eq!(format_denomination_msat(1024), "1024 msat");
        assert_eq!(format_denomination_msat(1_048_576), "1048 sats");
        assert_eq!(format_denomination_msat(1024_000), "1024 sats");
    }
}
