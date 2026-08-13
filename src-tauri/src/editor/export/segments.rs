use serde::{Deserialize, Serialize};

/// Endpoints must land within this many seconds of a real keyframe (or 0 /
/// the clip duration) to be accepted. Far tighter than the 1s keyframe
/// cadence the video pipeline produces, so it only ever absorbs float
/// round-trip noise from the frontend's own snapping, never a genuinely
/// off-keyframe request.
pub(crate) const SNAP_TOLERANCE_SECONDS: f64 = 0.001;

/// One kept (i.e. NOT cut) range, in source-timeline seconds. The
/// frontend already snapped both endpoints to keyframes; this is the wire
/// shape `export_trimmed_replay` deserializes straight from the command
/// call, and the same shape gets echoed into the exported bundle's
/// `keptSegments` metadata for provenance.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct KeepSegment {
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SegmentValidationFailure {
    Invalid,
}

/// Re-validates frontend-supplied kept ranges against the ACTUAL probed
/// keyframe list and duration — never trusts the frontend's own snapping.
/// Rejects (rather than silently re-encoding) unless every segment is
/// ordered, non-overlapping, and has both endpoints within
/// `SNAP_TOLERANCE_SECONDS` of a real keyframe or a clip boundary (0 /
/// duration, which are always safe cut/copy points even when they are not
/// themselves keyframe packets).
pub(crate) fn validate_keep_segments(
    requested: &[KeepSegment],
    keyframe_seconds: &[f64],
    duration_seconds: f64,
) -> Result<Vec<KeepSegment>, SegmentValidationFailure> {
    if requested.is_empty() {
        return Err(SegmentValidationFailure::Invalid);
    }
    for segment in requested {
        if segment.end - segment.start <= SNAP_TOLERANCE_SECONDS {
            return Err(SegmentValidationFailure::Invalid);
        }
        if !is_snap_point(segment.start, keyframe_seconds, duration_seconds)
            || !is_snap_point(segment.end, keyframe_seconds, duration_seconds)
        {
            return Err(SegmentValidationFailure::Invalid);
        }
    }
    for pair in requested.windows(2) {
        if pair[1].start + SNAP_TOLERANCE_SECONDS < pair[0].end {
            return Err(SegmentValidationFailure::Invalid);
        }
    }
    Ok(requested.to_vec())
}

fn is_snap_point(value: f64, keyframe_seconds: &[f64], duration_seconds: f64) -> bool {
    if value < -SNAP_TOLERANCE_SECONDS || value > duration_seconds + SNAP_TOLERANCE_SECONDS {
        return false;
    }
    near(value, 0.0)
        || near(value, duration_seconds)
        || keyframe_seconds.iter().any(|kf| near(value, *kf))
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= SNAP_TOLERANCE_SECONDS
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYFRAMES: [f64; 4] = [0.0, 1.0, 2.0, 3.0];
    const DURATION: f64 = 3.5;

    #[test]
    fn accepts_a_single_segment_snapped_to_keyframes() {
        let segments = vec![KeepSegment {
            start: 1.0,
            end: 3.0,
        }];
        assert_eq!(
            validate_keep_segments(&segments, &KEYFRAMES, DURATION),
            Ok(segments)
        );
    }

    #[test]
    fn accepts_the_clip_boundaries_even_when_not_keyframes() {
        let segments = vec![KeepSegment {
            start: 0.0,
            end: DURATION,
        }];
        assert!(validate_keep_segments(&segments, &KEYFRAMES, DURATION).is_ok());
    }

    #[test]
    fn accepts_two_non_overlapping_ordered_segments() {
        let segments = vec![
            KeepSegment {
                start: 0.0,
                end: 1.0,
            },
            KeepSegment {
                start: 2.0,
                end: 3.0,
            },
        ];
        assert!(validate_keep_segments(&segments, &KEYFRAMES, DURATION).is_ok());
    }

    #[test]
    fn rejects_an_empty_request() {
        assert_eq!(
            validate_keep_segments(&[], &KEYFRAMES, DURATION),
            Err(SegmentValidationFailure::Invalid)
        );
    }

    #[test]
    fn rejects_an_endpoint_off_any_keyframe() {
        let segments = vec![KeepSegment {
            start: 0.5,
            end: 2.0,
        }];
        assert_eq!(
            validate_keep_segments(&segments, &KEYFRAMES, DURATION),
            Err(SegmentValidationFailure::Invalid)
        );
    }

    #[test]
    fn rejects_overlapping_segments() {
        let segments = vec![
            KeepSegment {
                start: 0.0,
                end: 2.0,
            },
            KeepSegment {
                start: 1.0,
                end: 3.0,
            },
        ];
        assert_eq!(
            validate_keep_segments(&segments, &KEYFRAMES, DURATION),
            Err(SegmentValidationFailure::Invalid)
        );
    }

    #[test]
    fn rejects_a_zero_length_segment() {
        let segments = vec![KeepSegment {
            start: 1.0,
            end: 1.0,
        }];
        assert_eq!(
            validate_keep_segments(&segments, &KEYFRAMES, DURATION),
            Err(SegmentValidationFailure::Invalid)
        );
    }

    #[test]
    fn tolerates_sub_millisecond_float_noise() {
        let segments = vec![KeepSegment {
            start: 1.0 + 0.0005,
            end: 3.0 - 0.0005,
        }];
        assert!(validate_keep_segments(&segments, &KEYFRAMES, DURATION).is_ok());
    }

    #[test]
    fn rejects_an_endpoint_past_the_clip_duration() {
        let segments = vec![KeepSegment {
            start: 0.0,
            end: DURATION + 1.0,
        }];
        assert_eq!(
            validate_keep_segments(&segments, &KEYFRAMES, DURATION),
            Err(SegmentValidationFailure::Invalid)
        );
    }
}
