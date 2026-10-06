#[cfg(windows)]
pub mod cli;
pub mod controller;
pub mod guard;
#[cfg(windows)]
pub mod native;
#[cfg(windows)]
pub mod session;
pub mod startup;
#[cfg(windows)]
pub mod ui;
#[cfg(windows)]
pub mod watchdog;
pub type Ramp = Vec<Vec<u16>>;
/// Dimming amount, not brightness; reject instead of silently rounding IPC inputs.
pub fn dimming_percent(dim: i64) -> Result<u8, String> {
    if !(0..=90).contains(&dim) {
        return Err("dimming requires an integer from 0 to 90".into());
    }
    Ok((100 - dim) as u8)
}
pub fn scale(original: &Ramp, percent: u8) -> Result<Ramp, String> {
    if !(10..=100).contains(&percent)
        || original.len() != 3
        || original.iter().any(|c| c.len() != 256)
    {
        return Err("requires 10..100% and a 3x256 ramp".into());
    }
    Ok(original
        .iter()
        .map(|channel| {
            channel
                .iter()
                .map(|v| ((*v as u32 * percent as u32 + 50) / 100) as u16)
                .collect()
        })
        .collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsafe_range_and_malformed_ramps() {
        let ramp = vec![vec![65535; 256]; 3];
        for p in [0, 9, 101, 255] {
            assert!(scale(&ramp, p).is_err(), "accepted {p}");
        }
        assert!(scale(&vec![vec![0; 255]; 3], 50).is_err());
        assert!(scale(&vec![vec![0; 256]; 2], 50).is_err());
    }
    #[test]
    fn scales_original_not_identity() {
        let original = vec![vec![1000; 256], vec![2000; 256], vec![4000; 256]];
        let result = scale(&original, 50).unwrap();
        assert_eq!(result[0][100], 500);
        assert_eq!(result[1][100], 1000);
        assert_eq!(result[2][100], 2000);
        assert_eq!(scale(&original, 100).unwrap(), original);
    }
}
