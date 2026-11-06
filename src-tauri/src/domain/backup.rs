//! Which automatic backups to delete (BAK-3).

use chrono::{NaiveDate, NaiveDateTime};

/// The local time in every backup's file name, e.g. `2026-10-26_0930` (DATA_MODEL §8, BAK-1).
pub const STAMP: &str = "%Y-%m-%d_%H%M";

/// How many automatic backups are kept (BAK-3).
pub const KEEP: usize = 30;

/// Whether to warn about backups (BAK-7): the last one, on its local date, is more than 7 days
/// before `today`, or there's never been one. A date after today (a clock set back) isn't due.
pub fn is_due(last: Option<NaiveDate>, today: NaiveDate) -> bool {
    last.is_none_or(|last| (today - last).num_days() > 7)
}

/// The files among `names` to delete so that only the newest `KEEP` backups remain. Only names
/// a backup gets, `<prefix>-YYYY-MM-DD_HHMM.db` or with `-2`, `-3`, … before `.db`, count:
/// anything else in the folder is never deleted. Newest goes by the time in the name, then
/// the number, never by the file's dates, which a copy can change. `just_made` always counts as
/// the newest, so a clock set back can't delete the backup it has just written.
pub fn to_prune(prefix: &str, names: &[String], just_made: &str) -> Vec<String> {
    let mut backups: Vec<((bool, NaiveDateTime, u32), &String)> = names
        .iter()
        .filter_map(|name| {
            let (time, number) = when(prefix, name)?;
            Some(((name == just_made, time, number), name))
        })
        .collect();
    backups.sort_by_key(|(rank, _)| std::cmp::Reverse(*rank));
    backups
        .into_iter()
        .skip(KEEP)
        .map(|(_, name)| name.clone())
        .collect()
}

/// The time in a backup's name and its same-minute number (1 when it has none).
fn when(prefix: &str, name: &str) -> Option<(NaiveDateTime, u32)> {
    let rest = name.strip_prefix(prefix)?.strip_prefix('-')?;
    let (stamp, more) = rest.strip_suffix(".db")?.split_at_checked(15)?;
    let number = if more.is_empty() {
        1
    } else {
        let digits = more.strip_prefix('-')?;
        // Only what a backup is named: `-2`, `-3`, … with no `+`, leading zero or `-1`, so no two
        // names rank the same.
        if !digits.bytes().all(|b| b.is_ascii_digit()) || digits.starts_with('0') {
            return None;
        }
        digits.parse().ok().filter(|n| *n >= 2)?
    };
    let time = NaiveDateTime::parse_from_str(stamp, STAMP).ok()?;
    Some((time, number))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// `count` daily backups, newest first, ending on 2026-11-03.
    fn days(count: u32) -> Vec<String> {
        (0..count)
            .map(|i| {
                let day = chrono::NaiveDate::from_ymd_opt(2026, 11, 3).unwrap()
                    - chrono::Days::new(i.into());
                format!("jv-{}_0905.db", day.format("%Y-%m-%d"))
            })
            .collect()
    }

    #[test]
    fn a_backup_is_due_after_more_than_7_days_or_with_none_at_all() {
        let day = |d| chrono::NaiveDate::from_ymd_opt(2026, 11, d).unwrap();
        let oct = |d| chrono::NaiveDate::from_ymd_opt(2026, 10, d).unwrap();
        assert!(is_due(None, day(3)));
        assert!(!is_due(Some(day(3)), day(3)));
        assert!(!is_due(Some(oct(27)), day(3)), "7 days");
        assert!(is_due(Some(oct(26)), day(3)), "8 days");
        assert!(!is_due(Some(day(10)), day(3)), "a clock set back");
    }

    #[test]
    fn thirty_or_fewer_are_all_kept() {
        assert!(to_prune("jv", &days(0), "").is_empty());
        assert!(to_prune("jv", &days(30), "").is_empty());
    }

    #[test]
    fn only_the_newest_thirty_are_kept() {
        let all = days(33);
        let mut shuffled = all.clone();
        shuffled.reverse();
        shuffled.swap(0, 17);
        let mut pruned = to_prune("jv", &shuffled, &all[0]);
        pruned.sort();
        let mut oldest = all[30..].to_vec();
        oldest.sort();
        assert_eq!(pruned, oldest);
    }

    #[test]
    fn the_time_in_the_name_decides_across_months_and_years() {
        let mut list = days(29);
        list.push("jv-2025-12-31_2359.db".into());
        list.push("jv-2026-01-01_0000.db".into());
        assert_eq!(to_prune("jv", &list, ""), ["jv-2025-12-31_2359.db"]);
    }

    #[test]
    fn a_same_minute_copy_is_newer_than_the_first() {
        let mut list = days(29);
        list.push("jv-2020-01-01_0000-2.db".into());
        list.push("jv-2020-01-01_0000.db".into());
        list.push("jv-2020-01-01_0000-10.db".into());
        let mut pruned = to_prune("jv", &list, "");
        pruned.sort();
        assert_eq!(pruned, ["jv-2020-01-01_0000-2.db", "jv-2020-01-01_0000.db"]);
    }

    #[test]
    fn the_backup_just_made_is_kept_even_if_the_clock_went_back() {
        let mut list = days(30); // Oct 5 to Nov 3
        list.push("jv-2026-01-01_0900.db".into());
        assert_eq!(
            to_prune("jv", &list, "jv-2026-01-01_0900.db"),
            ["jv-2026-10-05_0905.db"]
        );
    }

    #[test]
    fn other_files_are_never_pruned() {
        let mut list = days(30);
        list.extend(names(&[
            "jv-2020-01-01_0000.db.tmp",
            "jv-2020-01-01_0000.db-wal",
            "jv-2020-01-01_0000-.db",
            "jv-2020-01-01_0000-x.db",
            "jv-2020-01-01_0000-+2.db",
            "jv-2020-01-01_0000-1.db",
            "jv-2020-01-01_0000-02.db",
            "jv-2020-01-01_000.db",
            "jv-2020-13-01_0000.db",
            "jv-pre-migration-v1-2020-01-01_0000.db",
            "other-2020-01-01_0000.db",
            "jvx-2020-01-01_0000.db",
            "notes.txt",
        ]));
        assert!(to_prune("jv", &list, "").is_empty());
        // The real prefix in a debug build contains a dash.
        let dev = names(&["jobertsvault-dev-2020-01-01_0000.db"]);
        assert!(to_prune("jobertsvault", &dev, "").is_empty());
    }
}
