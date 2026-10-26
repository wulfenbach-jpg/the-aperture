//! Disclosed-confidence statistics for paired/rotated evaluation.
//!
//! Method (documented on the report): per seed d, the two rotation games are
//! treated as ONE correlated unit — the per-seed paired difference is the mean
//! of the rotation games' search-minus-fair-odds scores, so rotations of the
//! same seed are never counted as independent draws. The reported interval is a
//! Student-t interval over the per-seed paired differences. Because denominators
//! are small, disclosure of n and the interval width is mandatory.

/// One game's search result at fair odds: win +1, draw 0.5, loss 0. Paired
/// differences are computed against fair odds, so a draw contributes 0.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Go {
    Win,
    Draw,
    Loss,
}

impl Go {
    pub fn score(self) -> f64 {
        match self {
            Go::Win => 1.0,
            Go::Draw => 0.5,
            Go::Loss => 0.0,
        }
    }
}

/// The paired sample: per-seed mean score over that seed's rotation games.
pub struct PairedSample {
    pub per_seed: Vec<f64>,
}

impl PairedSample {
    /// Mean per-seed difference from fair odds (`score() - 0.5` averaged).
    /// The t interval is centered on this mean; a mean above 0 means the search
    /// policy outscored fair odds. n = number of seeds (one correlated unit
    /// each). Callers must pre-subtract 0.5; this struct only averages.
    pub fn mean(&self) -> f64 {
        self.per_seed.iter().sum::<f64>() / self.per_seed.len() as f64
    }

    /// Sample standard deviation (ddof = 1). Errors when fewer than 2 seeds.
    pub fn sd(&self) -> Result<f64, String> {
        let n = self.per_seed.len();
        if n < 2 {
            return Err(format!(
                "a t interval needs at least 2 paired units (seeds), got {n}"
            ));
        }
        let mean = self.mean();
        let var = self
            .per_seed
            .iter()
            .map(|x| (x - mean) * (x - mean))
            .sum::<f64>()
            / (n - 1) as f64;
        Ok(var.sqrt())
    }

    /// Student-t interval for the true mean paired difference at 95%.
    /// `df < 30` uses a pinned two-sided 97.5th percentile table; larger df use
    /// the normal approximation and say so in the returned note.
    pub fn t_interval_95(&self) -> Result<(f64, f64, String), String> {
        let n = self.per_seed.len();
        if n < 2 {
            return Err(format!(
                "a t interval needs at least 2 paired units (seeds), got {n}"
            ));
        }
        let (t, note) = t_025(n - 1);
        let half = t * self.sd()? / (n as f64).sqrt();
        let mean = self.mean();
        Ok((mean - half, mean + half, note))
    }

    pub fn mean_opt(&self) -> Option<f64> {
        if self.per_seed.is_empty() {
            None
        } else {
            Some(self.mean())
        }
    }

    /// Disclosed numerator/denominator for review, e.g.
    /// "paired units n=10; scored terminal instances=20; no excluded seeds".
    pub fn denominator_note(&self, scored_instances: usize, excluded: &[u64]) -> String {
        let excluded_note = if excluded.is_empty() {
            "no excluded seeds".to_string()
        } else {
            format!("excluded seeds (no scored game): {excluded:?}")
        };
        format!(
            "paired units n={n}; scored terminal instances={scored_instances}; {excluded_note}",
            n = self.per_seed.len(),
        )
    }
}

/// Two-sided 97.5th percentile of Student-t for small df, pinned table.
pub fn t_025(df: usize) -> (f64, String) {
    let table: [(usize, f64); 15] = [
        (1, 12.706),
        (2, 4.303),
        (3, 3.182),
        (4, 2.776),
        (5, 2.571),
        (6, 2.447),
        (7, 2.365),
        (8, 2.306),
        (9, 2.262),
        (10, 2.228),
        (11, 2.201),
        (12, 2.179),
        (14, 2.145),
        (16, 2.120),
        (18, 2.101),
    ];
    if let Some(&(d, t)) = table.iter().find(|&&(d, _)| d == df) {
        (t, format!("pinned t table, df={d}"))
    } else if df > 18 && df < 30 {
        // Conservative: use the largest table value below instead of a guess.
        (
            2.262,
            format!("df={df} between pinned rows; conservative t=2.262 used"),
        )
    } else {
        (
            1.96,
            format!("df={df} is large; normal approximation t=1.96 used"),
        )
    }
}

/// Wilson score interval on a simple proportion. Used only as a secondary
/// readout on TERMINAL games; the paired analysis is the released interval.
/// Numerator = wanted wins, denominator = terminal games, must be > 0.
pub fn wilson_95(wins: usize, total: usize) -> Result<(f64, f64), String> {
    if total == 0 {
        return Err("Wilson interval needs at least one terminal game".to_string());
    }
    let z = 1.96;
    let p = wins as f64 / total as f64;
    let n = total as f64;
    let denom = 1.0 + z * z / n;
    let centre = (p + z * z / (2.0 * n)) / denom;
    let half = z * ((p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt()) / denom;
    Ok(((centre - half).max(0.0), (centre + half).min(1.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_interval_is_symmetric_around_mean() {
        let s = PairedSample {
            per_seed: vec![0.5, 1.0, 0.0, 0.5, 1.0, 0.5],
        };
        let (lo, hi, note) = s.t_interval_95().unwrap();
        assert!((lo + hi) / 2.0 == s.mean());
        assert!(lo < s.mean() && s.mean() < hi);
        assert!(note.contains("pinned t table, df=5"));
    }

    #[test]
    fn one_seed_cannot_buy_an_interval() {
        let s = PairedSample {
            per_seed: vec![1.0],
        };
        assert!(s.t_interval_95().is_err());
        assert!(s.sd().is_err());
    }

    #[test]
    fn wilson_bounds_stay_in_zero_one() {
        let (lo, hi) = wilson_95(4, 4).unwrap();
        assert!((hi - 1.0).abs() < 1e-12 && lo > 0.3);
        assert!(wilson_95(1, 0).is_err());
    }

    #[test]
    fn go_scores_match_fair_odds() {
        assert_eq!(Go::Win.score(), 1.0);
        assert_eq!(Go::Draw.score(), 0.5);
        assert_eq!(Go::Loss.score(), 0.0);
    }

    /// Raw per-game scores (like 0.30, i.e. BELOW fair odds) translate into
    /// NEGATIVE paired differences from fair odds, so any interval around them
    /// sits below zero: a raw mean below 0.5 can never satisfy the released
    /// "paired interval lower bound above parity" gate (review B2).
    #[test]
    fn raw_scores_below_fair_odds_give_negative_diffs() {
        // Raw per-game scores 0.30-ish (i.e. below 0.5), pre-differenced.
        let scores = [0.3, 0.3, 0.0];
        let sample = PairedSample {
            per_seed: scores.iter().map(|s| s - 0.5).collect(),
        };
        let mean = sample.mean();
        assert!((mean - (-0.3)).abs() < 1e-6, "mean diff {mean}");
        let (lo, _hi, _) = sample.t_interval_95().unwrap();
        assert!(lo < 0.0);
    }
}
