//! Timeline planning, coverage calculation and authority-local video persistence.
mod local;
use crate::{Error, Result};
pub use local::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interval {
    pub start_ms: u64,
    pub end_ms: u64,
}
pub fn plan_segments(duration_ms: u64, segment_ms: u64, overlap_ms: u64) -> Result<Vec<Interval>> {
    if duration_ms == 0 || duration_ms > 86_400_000 || segment_ms == 0 || overlap_ms >= segment_ms {
        return Err(Error::Invalid("video segmentation parameters"));
    }
    let mut result = Vec::new();
    let mut start: u64 = 0;
    loop {
        let end = start.saturating_add(segment_ms).min(duration_ms);
        result.push(Interval {
            start_ms: start,
            end_ms: end,
        });
        if result.len() > 10_000 {
            return Err(Error::Invalid("too many segments"));
        }
        if end == duration_ms {
            break;
        }
        start = end - overlap_ms;
    }
    Ok(result)
}
pub fn coverage_ms(duration_ms: u64, intervals: &[Interval]) -> Result<u64> {
    if duration_ms == 0 || duration_ms > 86_400_000 || intervals.len() > 100_000 {
        return Err(Error::Invalid("coverage bounds"));
    }
    let mut sorted = intervals.to_vec();
    sorted.sort_by_key(|i| (i.start_ms, i.end_ms));
    let mut covered = 0;
    let mut end = 0;
    for i in sorted {
        if i.start_ms >= i.end_ms || i.end_ms > duration_ms {
            return Err(Error::Invalid("coverage interval"));
        }
        if i.end_ms > end {
            covered += i.end_ms - i.start_ms.max(end);
            end = i.end_ms;
        }
    }
    Ok(covered)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisCoverage {
    pub duration_ms: u64,
    pub visual: Vec<Interval>,
    pub audio: Vec<Interval>,
    pub uncertain: Vec<Interval>,
    pub synthesis_saved: bool,
}
impl AnalysisCoverage {
    pub fn is_complete(&self) -> Result<bool> {
        Ok(
            coverage_ms(self.duration_ms, &self.visual)? == self.duration_ms
                && coverage_ms(self.duration_ms, &self.audio)? == self.duration_ms
                && self.uncertain.is_empty()
                && self.synthesis_saved,
        )
    }
    /// Intersection coverage prevents disjoint visual/audio work from showing 100%.
    pub fn analyzed_ms(&self) -> Result<u64> {
        let visual = coverage_ms(self.duration_ms, &self.visual)?;
        let audio = coverage_ms(self.duration_ms, &self.audio)?;
        let mut union = self.visual.clone();
        union.extend_from_slice(&self.audio);
        Ok(visual + audio - coverage_ms(self.duration_ms, &union)?)
    }
}
