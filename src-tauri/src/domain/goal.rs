use chrono::{Datelike, Months, NaiveDate};

use crate::domain::forecast::{Movement, PayWindow, running};
use crate::domain::money::Money;

/// Calendar months from `today` to `target`, a part month counting as a whole one: Nov 12 →
/// Dec 12 is 1, Dec 13 is 2. 0 once the target date is today or has passed (DATA_MODEL §5.7).
pub fn months_left(today: NaiveDate, target: NaiveDate) -> i64 {
    if target <= today {
        return 0;
    }
    let whole = i64::from(target.year() - today.year()) * 12 + i64::from(target.month0())
        - i64::from(today.month0());
    // Adding months clamps to the month's end, so Jan 31 + 1 month is Feb 28.
    let reached = u32::try_from(whole)
        .ok()
        .and_then(|n| today.checked_add_months(Months::new(n)))
        .is_some_and(|date| date >= target);
    if reached { whole } else { whole + 1 }
}

/// What must be saved each month to cover `remaining` in `months`, rounded up to the minor
/// unit so saving it every month always reaches the target. None when nothing is left to save
/// or no month is (GOL-3).
pub fn monthly_needed(remaining: Money, months: i64) -> Option<Money> {
    // Divides first, so no amount can overflow on the way up.
    (remaining > Money::ZERO && months > 0)
        .then(|| Money(remaining.0 / months + i64::from(remaining.0 % months != 0)))
}

/// Another active goal that isn't achieved and has a target date: it takes `monthly_needed` on each
/// of its first `months_left` saving dates (§5.7, DATA_MODEL §5.17).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OtherGoal {
    pub monthly_needed: Money,
    pub months_left: i64,
}

/// What a goal could count on each month, and its earliest realistic target date: none while
/// something is left to save and nothing can be set aside (DATA_MODEL §5.17).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Room {
    pub monthly_room: Money,
    pub earliest: Option<NaiveDate>,
}

/// `today + k` months for k = 1, 2, … up to `end`, each clamped to its month's end, as §5.7
/// counts months left.
pub fn saving_dates(today: NaiveDate, end: NaiveDate) -> Vec<NaiveDate> {
    (1..)
        .map_while(|k| today.checked_add_months(Months::new(k)))
        .take_while(|date| *date <= end)
        .collect()
}

/// The monthly room: the most that can go into the goal's account on every saving date without
/// any day's running total going below 0, after the other goals take theirs. 0 when any day of
/// the horizon already runs short. `None` with no pay windows (no payday rule).
pub fn room(
    money_now: Money,
    today: NaiveDate,
    windows: &[PayWindow],
    movements: &[Movement],
    others: &[OtherGoal],
    left_to_save: Money,
) -> Option<Room> {
    let end = windows.last()?.end;
    let steps = running(money_now, today, end, movements);
    let saving = saving_dates(today, end);
    let first = *saving.first()?;
    // Every share stays the same between the days the total or the saving dates change.
    let mut days: Vec<NaiveDate> = steps
        .iter()
        .map(|s| s.0)
        .chain(saving.iter().copied())
        .filter(|day| *day >= first)
        .collect();
    days.sort_unstable();
    days.dedup();
    let short = steps.iter().any(|s| s.1 < Money::ZERO);
    let lowest = days
        .iter()
        .map(|&day| {
            let run = steps
                .iter()
                .take_while(|s| s.0 <= day)
                .last()
                .map_or(0, |s| s.1.0);
            let n = saving.iter().take_while(|s| **s <= day).count() as i64;
            let taken = others.iter().fold(0_i64, |sum, g| {
                sum.saturating_add(
                    g.monthly_needed
                        .0
                        .saturating_mul(n.min(g.months_left.max(0))),
                )
            });
            run.saturating_sub(taken).div_euclid(n)
        })
        .min()
        .unwrap_or(0);
    let monthly_room = Money(if short { 0 } else { lowest.max(0) });
    Some(Room {
        monthly_room,
        earliest: earliest(today, left_to_save, monthly_room),
    })
}

/// `today + ⌈left ÷ room⌉` months, clamped to the month's end; today when nothing is left. None
/// when nothing can be set aside, or the date is out of range.
pub fn earliest(today: NaiveDate, left_to_save: Money, monthly_room: Money) -> Option<NaiveDate> {
    if left_to_save <= Money::ZERO {
        return Some(today);
    }
    if monthly_room <= Money::ZERO {
        return None;
    }
    let (left, room) = (left_to_save.0, monthly_room.0);
    let months = left / room + i64::from(left % room != 0);
    u32::try_from(months)
        .ok()
        .and_then(|m| today.checked_add_months(Months::new(m)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::forecast::{Direction, Source, horizon_end};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("a real test date")
    }

    fn left(target: NaiveDate) -> i64 {
        months_left(d(2026, 11, 12), target)
    }

    #[test]
    fn a_part_month_counts_as_a_whole_one() {
        assert_eq!(left(d(2026, 11, 13)), 1);
        assert_eq!(left(d(2026, 11, 30)), 1);
        assert_eq!(left(d(2026, 12, 12)), 1);
        assert_eq!(left(d(2026, 12, 13)), 2);
        assert_eq!(left(d(2027, 5, 12)), 6);
        assert_eq!(left(d(2027, 5, 13)), 7);
    }

    #[test]
    fn a_target_today_or_past_leaves_no_months() {
        assert_eq!(left(d(2026, 11, 12)), 0);
        assert_eq!(left(d(2026, 11, 11)), 0);
        assert_eq!(left(d(2025, 1, 1)), 0);
    }

    #[test]
    fn month_ends_clamp_and_leap_years_count() {
        // Jan 31 + 1 month is Feb 28, so Feb 28 is reached in one month and Mar 1 needs two.
        assert_eq!(months_left(d(2026, 1, 31), d(2026, 2, 28)), 1);
        assert_eq!(months_left(d(2026, 1, 31), d(2026, 3, 1)), 2);
        assert_eq!(months_left(d(2028, 1, 31), d(2028, 2, 29)), 1);
        assert_eq!(months_left(d(2026, 11, 30), d(2026, 12, 1)), 1);
        assert_eq!(months_left(d(2026, 12, 31), d(2027, 1, 1)), 1);
    }

    #[test]
    fn the_monthly_amount_is_rounded_up() {
        // 2,600.00 over 6 months is 433.333…, so 433.34 a month (UI_FLOWS §8.4's example).
        assert_eq!(monthly_needed(Money(260_000), 6), Some(Money(43_334)));
        assert_eq!(monthly_needed(Money(260_000), 2), Some(Money(130_000)));
        assert_eq!(monthly_needed(Money(1), 12), Some(Money(1)));
        assert_eq!(monthly_needed(Money::MAX, 1), Some(Money::MAX));
        assert_eq!(
            monthly_needed(Money(i64::MAX), 2),
            Some(Money(i64::MAX / 2 + 1))
        );
    }

    const TODAY: (i32, u32, u32) = (2026, 11, 3);

    fn today() -> NaiveDate {
        d(TODAY.0, TODAY.1, TODAY.2)
    }

    fn pay(date: NaiveDate, amount: i64) -> Movement {
        Movement {
            date,
            amount: Money(amount),
            direction: Direction::In,
            source: Source::Payday(1),
        }
    }

    fn bill(date: NaiveDate, amount: i64) -> Movement {
        Movement {
            direction: Direction::Out,
            source: Source::Recurring(2),
            ..pay(date, amount)
        }
    }

    /// One window to the horizon's end, as with a payday rule.
    fn windows() -> Vec<PayWindow> {
        vec![PayWindow {
            start: today(),
            end: horizon_end(today()).unwrap(),
        }]
    }

    /// 5,000.00 on the 15th of every month from Nov 15.
    fn pays() -> Vec<Movement> {
        (0..12)
            .map(|k| pay(d(2026, 11, 15) + Months::new(k), 500_000))
            .collect()
    }

    fn room_with(movements: &[Movement], others: &[OtherGoal], left: i64) -> Room {
        room(
            Money(1_000_000),
            today(),
            &windows(),
            movements,
            others,
            Money(left),
        )
        .unwrap()
    }

    #[test]
    fn the_room_is_the_lowest_share_over_the_saving_dates() {
        // From 10,000.00 now, +5,000.00 each 15th: the tightest day is Oct 3 (11 saving dates,
        // 65,000.00 by then): 590,909 a month.
        assert_eq!(room_with(&pays(), &[], 1).monthly_room, Money(590_909));
    }

    #[test]
    fn a_bill_later_in_the_month_lowers_the_room() {
        // A 12,000.00 bill on Dec 20 leaves 8,000.00, which must last two saving dates by Jan 3:
        // 4,000.00 a month, not the year's average surplus.
        let mut ms = pays();
        ms.push(bill(d(2026, 12, 20), 1_200_000));
        assert_eq!(room_with(&ms, &[], 1).monthly_room, Money(400_000));
    }

    #[test]
    fn another_dated_goal_takes_its_share_for_its_months_left() {
        // 1,000.00 a month for 3 months: the tightest day is still Oct 3, with 3,000.00 taken:
        // (65,000.00 − 3,000.00) ÷ 11.
        let other = OtherGoal {
            monthly_needed: Money(100_000),
            months_left: 3,
        };
        assert_eq!(room_with(&pays(), &[other], 1).monthly_room, Money(563_636));
        // One whose date has passed asks for nothing.
        let passed = OtherGoal {
            months_left: 0,
            ..other
        };
        assert_eq!(
            room_with(&pays(), &[passed], 1).monthly_room,
            Money(590_909)
        );
    }

    #[test]
    fn any_short_day_gives_a_room_of_0() {
        // Short on Nov 10, before the first saving date, even though pay brings it back.
        let mut ms = pays();
        ms.push(bill(d(2026, 11, 10), 1_100_000));
        let r = room_with(&ms, &[], 1);
        assert_eq!((r.monthly_room, r.earliest), (Money::ZERO, None));
        // Every peso spoken for by another goal is 0 too, never below.
        let greedy = OtherGoal {
            monthly_needed: Money(10_000_000),
            months_left: 12,
        };
        assert_eq!(room_with(&pays(), &[greedy], 1).monthly_room, Money::ZERO);
    }

    #[test]
    fn nothing_left_to_save_suggests_today() {
        let mut ms = pays();
        ms.push(bill(d(2026, 11, 10), 1_100_000));
        assert_eq!(room_with(&ms, &[], 0).earliest, Some(today()));
        assert_eq!(room_with(&pays(), &[], -5).earliest, Some(today()));
    }

    #[test]
    fn the_earliest_date_is_whole_months_of_the_room() {
        let r = room_with(&pays(), &[], 1_000_000);
        // 1,000,000 ÷ 590,909 is 1.7: 2 months.
        assert_eq!(r.earliest, Some(d(2027, 1, 3)));
    }

    #[test]
    fn saving_dates_clamp_to_the_months_end() {
        let jan31 = d(2027, 1, 31);
        let dates = saving_dates(jan31, horizon_end(jan31).unwrap());
        assert_eq!(dates[..3], [d(2027, 2, 28), d(2027, 3, 31), d(2027, 4, 30)]);
        assert_eq!(dates.len(), 11);
        assert_eq!(
            earliest(jan31, Money(100), Money(100)),
            Some(d(2027, 2, 28))
        );
        assert_eq!(
            earliest(jan31, Money(101), Money(100)),
            Some(d(2027, 3, 31))
        );
        assert_eq!(
            earliest(d(2028, 1, 31), Money(1), Money(1)),
            Some(d(2028, 2, 29))
        );
    }

    #[test]
    fn a_date_past_the_horizon_keeps_the_same_room() {
        assert_eq!(
            earliest(today(), Money(2_000_000), Money(100_000)),
            Some(d(2028, 7, 3))
        );
        assert_eq!(
            earliest(today(), Money(2_000_001), Money(100_000)),
            Some(d(2028, 8, 3))
        );
    }

    #[test]
    fn an_unreachable_date_is_none() {
        assert_eq!(earliest(today(), Money(i64::MAX), Money(1)), None);
        assert_eq!(earliest(today(), Money(1), Money::ZERO), None);
    }

    #[test]
    fn saving_monthly_needed_for_the_earliest_date_never_exceeds_the_room() {
        for start in [today(), d(2026, 1, 31), d(2028, 2, 29)] {
            for left in [1, 7, 99, 260_000, 1_000_000, 999_999_999_999] {
                for room in [1, 3, 43_334, 100_000, 999_999] {
                    let Some(e) = earliest(start, Money(left), Money(room)) else {
                        continue;
                    };
                    let needed = monthly_needed(Money(left), months_left(start, e));
                    assert!(needed <= Some(Money(room)), "{start} {left} {room}");
                }
            }
        }
    }

    #[test]
    fn no_windows_means_no_room() {
        assert_eq!(room(Money(1), today(), &[], &[], &[], Money(1)), None);
    }

    #[test]
    fn nothing_left_or_no_months_has_no_monthly_amount() {
        assert_eq!(monthly_needed(Money::ZERO, 6), None);
        assert_eq!(monthly_needed(Money(500), 0), None);
    }
}
