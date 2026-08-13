use crate::capture::SegmentBoundary;

pub const SEGMENT_MICROS: i64 = 10_000_000;
const DISCONTINUITY_MICROS: i64 = 2_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameStamp {
    pub media_micros: i64,
    pub session_micros: u64,
    pub boundary: Option<SegmentBoundary>,
}

#[derive(Default)]
pub struct Timeline {
    source_id: Option<String>,
    geometry: Option<(u32, u32)>,
    last_clock_micros: Option<i64>,
    segment_clock_start: i64,
    segment_session_start: u64,
    last_media_micros: i64,
}

impl Timeline {
    pub fn observe(
        &mut self,
        source_id: &str,
        geometry: (u32, u32),
        pts_value: i64,
        pts_timescale: i32,
        arrival_micros: u64,
    ) -> FrameStamp {
        let candidate = pts_micros(pts_value, pts_timescale)
            .unwrap_or_else(|| i64::try_from(arrival_micros).unwrap_or(i64::MAX));
        let boundary = self.boundary(source_id, geometry, candidate);
        if boundary.is_some() || self.source_id.is_none() {
            self.source_id = Some(source_id.to_owned());
            self.geometry = Some(geometry);
            self.segment_clock_start = candidate;
            self.segment_session_start = arrival_micros;
            self.last_media_micros = 0;
        }

        let mut media_micros = candidate.saturating_sub(self.segment_clock_start);
        if media_micros <= self.last_media_micros && self.last_clock_micros.is_some() {
            media_micros = self.last_media_micros.saturating_add(1);
        }
        self.last_clock_micros = Some(candidate);
        self.last_media_micros = media_micros;

        FrameStamp {
            media_micros,
            session_micros: arrival_micros,
            boundary,
        }
    }

    pub fn segment_session_start(&self) -> u64 {
        self.segment_session_start
    }

    fn boundary(
        &self,
        source_id: &str,
        geometry: (u32, u32),
        candidate: i64,
    ) -> Option<SegmentBoundary> {
        let current_source = self.source_id.as_deref()?;
        if current_source != source_id {
            return Some(SegmentBoundary::SourceChanged);
        }
        if self.geometry != Some(geometry) {
            return Some(SegmentBoundary::GeometryChanged);
        }
        let previous = self.last_clock_micros?;
        let delta = candidate.saturating_sub(previous);
        if delta.unsigned_abs() > DISCONTINUITY_MICROS as u64 {
            return Some(SegmentBoundary::ClockDiscontinuity);
        }
        if candidate.saturating_sub(self.segment_clock_start) >= SEGMENT_MICROS {
            return Some(SegmentBoundary::Duration);
        }
        None
    }
}

fn pts_micros(value: i64, timescale: i32) -> Option<i64> {
    if value < 0 || timescale <= 0 {
        return None;
    }
    let micros = i128::from(value)
        .checked_mul(1_000_000)?
        .checked_div(i128::from(timescale))?;
    i64::try_from(micros).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_and_small_regression_are_strictly_monotonic() {
        let mut timeline = Timeline::default();
        let first = timeline.observe("display:1", (1920, 1080), 10, 30, 0);
        let duplicate = timeline.observe("display:1", (1920, 1080), 10, 30, 1);
        let regression = timeline.observe("display:1", (1920, 1080), 9, 30, 2);
        assert_eq!(first.media_micros, 0);
        assert_eq!(duplicate.media_micros, 1);
        assert_eq!(regression.media_micros, 2);
        assert_eq!(regression.boundary, None);
    }

    #[test]
    fn missing_pts_uses_monotonic_arrival_time() {
        let mut timeline = Timeline::default();
        assert_eq!(
            timeline
                .observe("display:1", (800, 600), -1, 0, 25_000)
                .media_micros,
            0
        );
        assert_eq!(
            timeline
                .observe("display:1", (800, 600), -1, 0, 58_000)
                .media_micros,
            33_000
        );
    }

    #[test]
    fn variable_pacing_preserves_media_time() {
        let mut timeline = Timeline::default();
        for expected in [0, 33_333, 100_000, 116_667] {
            let stamp = timeline.observe(
                "display:1",
                (800, 600),
                expected,
                1_000_000,
                expected as u64,
            );
            assert_eq!(stamp.media_micros, expected);
            assert_eq!(stamp.boundary, None);
        }
    }

    #[test]
    fn duration_source_geometry_and_clock_create_boundaries() {
        let mut timeline = Timeline::default();
        timeline.observe("display:1", (800, 600), 0, 1_000_000, 0);
        for second in 1..10 {
            timeline.observe(
                "display:1",
                (800, 600),
                second * 1_000_000,
                1_000_000,
                second as u64,
            );
        }
        let duration = timeline.observe("display:1", (800, 600), SEGMENT_MICROS, 1_000_000, 10);
        assert_eq!(duration.boundary, Some(SegmentBoundary::Duration));
        let source = timeline.observe("window:2", (800, 600), 10_100_000, 1_000_000, 20);
        assert_eq!(source.boundary, Some(SegmentBoundary::SourceChanged));
        let geometry = timeline.observe("window:2", (1024, 768), 10_200_000, 1_000_000, 30);
        assert_eq!(geometry.boundary, Some(SegmentBoundary::GeometryChanged));
        let regression = timeline.observe("window:2", (1024, 768), 7_900_000, 1_000_000, 40);
        assert_eq!(
            regression.boundary,
            Some(SegmentBoundary::ClockDiscontinuity)
        );
        let gap = timeline.observe("window:2", (1024, 768), 10_100_001, 1_000_000, 50);
        assert_eq!(gap.boundary, Some(SegmentBoundary::ClockDiscontinuity));
    }
}
