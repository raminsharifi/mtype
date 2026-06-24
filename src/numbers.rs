//! Scoring math, ported 1:1 from `packages/util/src/numbers.ts` and
//! `frontend/src/ts/utils/numbers.ts`. Kept separate and pure so it can be unit
//! tested against the reference TypeScript values.

/// `calculateWpm` - chars / 5 / minutes. A "word" is 5 characters.
pub fn calculate_wpm(char_count: f64, duration_seconds: f64) -> f64 {
    if duration_seconds <= 0.0 {
        return 0.0;
    }
    char_count / 5.0 / (duration_seconds / 60.0)
}

/// `roundTo2`
pub fn round_to_2(num: f64) -> f64 {
    ((num + f64::EPSILON) * 100.0).round() / 100.0
}

/// `mean` - average, or 0 for an empty slice (matches the TS try/catch fallback).
pub fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// `stdDev` - population standard deviation, 0 for empty (TS fallback).
pub fn std_dev(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let m = mean(xs);
    let var = xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / xs.len() as f64;
    var.sqrt()
}

/// `kogasa` - maps a coefficient of variation to a 0..100 consistency score.
pub fn kogasa(cov: f64) -> f64 {
    100.0 * (1.0 - (cov + cov.powi(3) / 3.0 + cov.powi(5) / 5.0).tanh())
}

/// Consistency over a series of speeds (the burst-per-word array Monkeytype
/// feeds into `kogasa`). Returns 0 when undefined/NaN, like the TS guard.
pub fn consistency(speeds: &[f64]) -> f64 {
    let m = mean(speeds);
    if m == 0.0 {
        return 0.0;
    }
    let c = round_to_2(kogasa(std_dev(speeds) / m));
    if c.is_nan() {
        0.0
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wpm_matches_reference() {
        // 250 correct chars in 60s -> 50 wpm
        assert_eq!(calculate_wpm(250.0, 60.0), 50.0);
        // 100 chars in 30s -> 40 wpm
        assert_eq!(calculate_wpm(100.0, 30.0), 40.0);
        assert_eq!(calculate_wpm(100.0, 0.0), 0.0);
    }

    #[test]
    fn round_to_2_matches() {
        // matches JS roundTo2: the +EPSILON nudge pushes 1.005 up to 1.01
        assert_eq!(round_to_2(1.005), 1.01);
        assert_eq!(round_to_2(2.675), 2.68);
        assert_eq!(round_to_2(50.12345), 50.12);
        assert_eq!(round_to_2(50.0), 50.0);
    }

    #[test]
    fn mean_and_stddev() {
        assert_eq!(mean(&[]), 0.0);
        assert_eq!(mean(&[2.0, 4.0, 6.0]), 4.0);
        assert_eq!(std_dev(&[]), 0.0);
        // population stddev of [2,4,6] = sqrt(8/3) ≈ 1.632993
        assert!((std_dev(&[2.0, 4.0, 6.0]) - 1.632_993_161_855_452).abs() < 1e-9);
    }

    #[test]
    fn kogasa_bounds() {
        // perfectly consistent -> 100
        assert_eq!(kogasa(0.0), 100.0);
        // higher variation -> lower score, monotonic, within (0,100)
        let a = kogasa(0.1);
        let b = kogasa(0.5);
        assert!(a > b);
        assert!(a < 100.0 && b > 0.0);
    }

    #[test]
    fn consistency_handles_empty_and_constant() {
        assert_eq!(consistency(&[]), 0.0);
        // constant speeds -> stddev 0 -> kogasa(0) = 100
        assert_eq!(consistency(&[50.0, 50.0, 50.0]), 100.0);
    }
}
