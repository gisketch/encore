mod lease;
mod segment;
mod store;

#[cfg(test)]
mod tests;

pub(crate) use lease::SegmentLease;
pub(crate) use segment::RetainedSegment;
pub(crate) use store::RollingStore;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionState {
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionSnapshot {
    pub state: RetentionState,
    pub minutes: u8,
    pub retained_segments: u64,
    pub retained_bytes: u64,
    pub error_code: Option<String>,
}

impl Default for RetentionSnapshot {
    fn default() -> Self {
        Self {
            state: RetentionState::Ready,
            minutes: 10,
            retained_segments: 0,
            retained_bytes: 0,
            error_code: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RetentionWindow {
    FiveMinutes,
    TenMinutes,
}

impl RetentionWindow {
    fn milliseconds(self) -> u64 {
        u64::from(self.minutes()) * 60 * 1_000
    }

    fn minutes(self) -> u8 {
        match self {
            Self::FiveMinutes => 5,
            Self::TenMinutes => 10,
        }
    }
}

impl TryFrom<u8> for RetentionWindow {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            5 => Ok(Self::FiveMinutes),
            10 => Ok(Self::TenMinutes),
            _ => Err("retention_duration_invalid"),
        }
    }
}
