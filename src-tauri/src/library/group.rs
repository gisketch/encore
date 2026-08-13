use super::scan::ScannedEntry;
use chrono::{Local, NaiveDate, TimeZone};
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub id: String,
    pub display_name: String,
    pub saved_at_unix_ms: u64,
    pub duration_seconds: Option<u64>,
    pub total_bytes: u64,
    pub trimmed: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryGroup {
    pub label: String,
    pub count: u32,
    pub total_bytes: u64,
    pub entries: Vec<LibraryEntry>,
}

/// Groups already-newest-first entries by local calendar day, labeling
/// each group `Today` / `Yesterday` / `Mon, Aug 10`. Kept in Rust (rather
/// than the frontend) so the harness's TypeScript complexity-1 ceiling
/// never has to host this branching — the Svelte layer just renders the
/// groups it's handed.
pub(crate) fn group_by_day(entries: Vec<ScannedEntry>, today: NaiveDate) -> Vec<LibraryGroup> {
    let mut groups: Vec<(NaiveDate, LibraryGroup)> = Vec::new();
    for entry in entries {
        let date = local_date(entry.saved_at_unix_ms);
        let entry = to_library_entry(entry);
        match groups.last_mut() {
            Some((last_date, group)) if *last_date == date => {
                group.count += 1;
                group.total_bytes += entry.total_bytes;
                group.entries.push(entry);
            }
            _ => groups.push((
                date,
                LibraryGroup {
                    label: day_label(date, today),
                    count: 1,
                    total_bytes: entry.total_bytes,
                    entries: vec![entry],
                },
            )),
        }
    }
    groups.into_iter().map(|(_, group)| group).collect()
}

fn to_library_entry(entry: ScannedEntry) -> LibraryEntry {
    LibraryEntry {
        id: entry.id,
        display_name: entry.display_name,
        saved_at_unix_ms: entry.saved_at_unix_ms,
        duration_seconds: entry.duration_seconds,
        total_bytes: entry.total_bytes,
        trimmed: entry.trimmed,
    }
}

fn local_date(saved_at_unix_ms: u64) -> NaiveDate {
    i64::try_from(saved_at_unix_ms)
        .ok()
        .and_then(|millis| Local.timestamp_millis_opt(millis).single())
        .unwrap_or_else(Local::now)
        .date_naive()
}

fn day_label(date: NaiveDate, today: NaiveDate) -> String {
    if date == today {
        "Today".to_string()
    } else if today.pred_opt() == Some(date) {
        "Yesterday".to_string()
    } else {
        date.format("%a, %b %-d").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, saved_at_unix_ms: u64, total_bytes: u64) -> ScannedEntry {
        ScannedEntry {
            id: id.to_string(),
            display_name: id.to_string(),
            saved_at_unix_ms,
            duration_seconds: None,
            total_bytes,
            trimmed: false,
        }
    }

    fn unix_ms(y: i32, m: u32, d: u32, h: u32, min: u32) -> u64 {
        let naive = chrono::NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap();
        let local = Local.from_local_datetime(&naive).single().unwrap();
        u64::try_from(local.timestamp_millis()).unwrap()
    }

    #[test]
    fn groups_same_day_entries_together_with_labels_and_rollups() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 13).unwrap();
        let entries = vec![
            entry("today-2", unix_ms(2026, 8, 13, 16, 0), 200),
            entry("today-1", unix_ms(2026, 8, 13, 9, 0), 100),
            entry("yesterday-1", unix_ms(2026, 8, 12, 10, 0), 50),
            entry("older-1", unix_ms(2026, 8, 10, 10, 0), 300),
        ];

        let groups = group_by_day(entries, today);

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].label, "Today");
        assert_eq!(groups[0].count, 2);
        assert_eq!(groups[0].total_bytes, 300);
        assert_eq!(groups[1].label, "Yesterday");
        assert_eq!(groups[1].count, 1);
        assert_eq!(groups[2].label, "Mon, Aug 10");
        assert_eq!(groups[2].count, 1);
    }

    #[test]
    fn empty_entries_produce_no_groups() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 13).unwrap();

        assert!(group_by_day(Vec::new(), today).is_empty());
    }
}
