//! LOD (Level-of-Detail) pyramid for min/max aggregation.
//!
//! Level 0 = raw samples (one MinMax per sample, min==max).
//! Level 1 = merge every 2 samples, length = N/2.
//! Level L = one entry, the global min/max of the whole dataset.
//! Total memory ≈ 2N × 8 bytes (f32 min + f32 max) ≈ 16N bytes.

/// One min/max pair representing a vertical span of sample values.

#[derive(Clone, Copy, Debug)]
pub struct MinMax {
    pub min: f32,
    pub max: f32,
}

impl MinMax {
    fn from_sample(val: f32) -> Self {
        Self { min: val, max: val }
    }

    /// Merge two MinMax into one that spans both ranges.
    fn merge(a: Self, b: Self) -> Self {
        Self {
            min: a.min.min(b.min),
            max: a.max.max(b.max),
        }
    }
}

/// A pre-computed LOD pyramid.
pub struct LodPyramid {
    /// levels[0] = original samples, levels[1] = every 2 merged, ...
    levels: Vec<Vec<MinMax>>,
    /// Total number of original samples.
    sample_count: usize,
}

impl LodPyramid {
    /// Build the pyramid from raw sample data.
    /// Time: O(N), Memory: ~16N bytes.
    pub fn from_samples(data: &[f32]) -> Result<Self, crate::error::Error> {
        let n = data.len();
        if n == 0 {
            return Err(crate::error::Error::EmptyData);
        }

        // Level 0: one MinMax per sample (min == max)
        let mut levels: Vec<Vec<MinMax>> = Vec::new();
        levels.push(data.iter().map(|&v| MinMax::from_sample(v)).collect());

        // Build higher levels by merging pairs
        loop {
            let Some(prev) = levels.last() else {
                return Err(crate::error::Error::Internal("levels unexpectedly empty"));
            };
            if prev.len() == 1 {
                break;
            }

            let next: Vec<MinMax> = prev
                .chunks(2)
                .map(|chunk| {
                    if chunk.len() == 2 {
                        MinMax::merge(chunk[0], chunk[1])
                    } else {
                        // Odd length: carry the last one up unchanged
                        chunk[0]
                    }
                })
                .collect();
            levels.push(next);
        }

        Ok(Self {
            levels,
            sample_count: n,
        })
    }

    /// Number of original samples.
    pub fn len(&self) -> usize {
        self.sample_count
    }

    /// Number of LOD levels (including level 0).
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Choose the appropriate LOD level index for a given
    /// (sample_range, target_bucket_count) combination.
    ///
    /// Picks the highest level where each entry covers <= samples_per_bucket.
    fn choose_level(&self, start: usize, end: usize, target_count: usize) -> usize {
        let range_len = end - start;
        if range_len == 0 || target_count == 0 {
            return 0;
        }

        // How many original samples fall into each target bucket?
        let sample_per_bucket = (range_len as f64 / target_count as f64).max(1.0);

        // Level L merges 2^L samples into one entry.
        // We want the highest level such that 2^level <= samples_per_bucket.
        let level = (sample_per_bucket.log2().floor() as usize).min(self.levels.len() - 1);
        level
    }

    /// Query the pyramid for `target_count` MinMax entries covering
    /// [start, end) sample indices.
    ///
    /// Returns a slice of MinMax entries — approximately `target_count`
    /// in length (may differ by at most 1 due to rounding).
    pub fn query(&self, start: usize, end: usize, target_count: usize) -> &[MinMax] {
        let start = start.min(self.sample_count.saturating_sub(1));
        let end = end.min(self.sample_count).max(start + 1);

        let level = self.choose_level(start, end, target_count);
        let bucket_size = 1usize << level; // how many original samples per level entry

        // Map [start, end) to indices on the chosen level
        let level_start = start >> level; // start / bucket_size
        let level_end = (end + bucket_size - 1) >> level; // ceil(end / bucket_size)
        let level_end = level_end.min(self.levels[level].len());
        &self.levels[level][level_start..level_end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_small_pyramid() -> Result<(), crate::error::Error> {
        // 4 samples: [0, 1, -1, 2]
        let data = vec![0.0, 1.0, -1.0, 2.0];
        let pyr = LodPyramid::from_samples(&data)?;

        assert_eq!(pyr.len(), 4);
        assert_eq!(pyr.level_count(), 3); // level0=4, level1=2, level2=1

        // Level 0: raw
        assert_eq!(pyr.levels[0].len(), 4);
        assert!((pyr.levels[0][0].min - 0.0).abs() < 1e-6);
        assert!((pyr.levels[0][0].max - 0.0).abs() < 1e-6);

        // Level 1: merged pairs: (0,1)=[0,1], (-1,2)=[-1,2]
        assert_eq!(pyr.levels[1].len(), 2);
        assert!((pyr.levels[1][0].min - 0.0).abs() < 1e-6);
        assert!((pyr.levels[1][0].max - 1.0).abs() < 1e-6);
        assert!((pyr.levels[1][1].min - (-1.0)).abs() < 1e-6);
        assert!((pyr.levels[1][1].max - 2.0).abs() < 1e-6);

        // Level 2: global min/max
        assert_eq!(pyr.levels[2].len(), 1);
        assert!((pyr.levels[2][0].min - (-1.0)).abs() < 1e-6);
        assert!((pyr.levels[2][0].max - 2.0).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn query_coarse_level() -> Result<(), crate::error::Error> {
        // 8 samples
        let data: Vec<f32> = (0..8).map(|i| i as f32).collect();
        let pyr = LodPyramid::from_samples(&data)?;

        // Query with target_count=2 → each bucket covers 4 samples → level 2
        let result = pyr.query(0, 8, 2);
        assert_eq!(result.len(), 2);
        // Level 2 (4x per entry): entries cover [0..4] and [4..8]
        assert!((result[0].min - 0.0).abs() < 1e-6);
        assert!((result[0].max - 3.0).abs() < 1e-6);
        assert!((result[1].min - 4.0).abs() < 1e-6);
        assert!((result[1].max - 7.0).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn query_subrange() -> Result<(), crate::error::Error> {
        let data: Vec<f32> = vec![0.0, 5.0, 2.0, 8.0, 1.0, 9.0, 3.0, 7.0];
        let pyr = LodPyramid::from_samples(&data)?;

        // Query only samples [2, 6): [2.0, 8.0, 1.0, 9.0]
        // target_count=2 → 4 samples / 2 = 2 per bucket → level 1
        let result = pyr.query(2, 6, 2);
        assert_eq!(result.len(), 2);
        // Level 1 entries: merge(2,8) → [2,8], merge(1,9) → [1,9]
        assert!((result[0].min - 2.0).abs() < 1e-6);
        assert!((result[0].max - 8.0).abs() < 1e-6);
        assert!((result[1].min - 1.0).abs() < 1e-6);
        assert!((result[1].max - 9.0).abs() < 1e-6);
        Ok(())
    }
}
