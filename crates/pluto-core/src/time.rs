//! Wall-clock date/time, computed from milliseconds since the Unix epoch.

/// Day of the week.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Weekday {
    Sunday = 0,
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
}

impl Weekday {
    /// Convert `0 = Sunday .. 6 = Saturday` into a [`Weekday`].
    pub fn from_index(i: u8) -> Weekday {
        match i % 7 {
            0 => Weekday::Sunday,
            1 => Weekday::Monday,
            2 => Weekday::Tuesday,
            3 => Weekday::Wednesday,
            4 => Weekday::Thursday,
            5 => Weekday::Friday,
            _ => Weekday::Saturday,
        }
    }
}

/// Month of the year.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Month {
    January = 1,
    February = 2,
    March = 3,
    April = 4,
    May = 5,
    June = 6,
    July = 7,
    August = 8,
    September = 9,
    October = 10,
    November = 11,
    December = 12,
}

impl Month {
    /// Convert a `1..=12` month number into a [`Month`].
    pub fn from_u8(m: u8) -> Month {
        match m {
            1 => Month::January,
            2 => Month::February,
            3 => Month::March,
            4 => Month::April,
            5 => Month::May,
            6 => Month::June,
            7 => Month::July,
            8 => Month::August,
            9 => Month::September,
            10 => Month::October,
            11 => Month::November,
            _ => Month::December,
        }
    }
}

/// Civil date and time, plus the epoch seconds it was derived from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTime {
    /// Seconds since the Unix epoch (UTC) this civil time corresponds to.
    pub secs: u64,
    /// Milliseconds within the current second (0..1000), for sub-second
    /// animations (blinking) that are faster than once per second.
    pub ms: u16,
    pub year: u16,
    pub month: Month,
    pub day: u8,
    pub weekday: Weekday,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl DateTime {
    /// Convert milliseconds since the Unix epoch (UTC) into a [`DateTime`].
    ///
    /// The algorithm for "days since epoch -> civil date" is the standard
    /// Howard Hinnant `civil_from_days`, valid for positive days.
    pub fn from_epoch_ms(ms: u64) -> Self {
        let seconds = ms / 1000;
        let days = (seconds / 86_400) as i64;
        let tod = seconds % 86_400;
        let hour = (tod / 3600) as u8;
        let minute = ((tod % 3600) / 60) as u8;
        let second = (tod % 60) as u8;

        let (year, month, day) = Self::civil_from_days(days);
        DateTime {
            secs: seconds,
            ms: (ms % 1000) as u16,
            year: year as u16,
            month: Month::from_u8(month),
            day,
            // 1970-01-01 was a Thursday (4 in the 0=Sunday..6=Saturday scheme).
            weekday: Weekday::from_index((days + 4).rem_euclid(7) as u8),
            hour,
            minute,
            second,
        }
    }

    fn civil_from_days(z: i64) -> (i64, u8, u8) {
        let z = z + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        let y = if m <= 2 { y + 1 } else { y };
        (y, m, d)
    }
}

/// True if `year` is a Gregorian leap year (divisible by 4, except for
/// centuries which must be divisible by 400).
pub const fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Number of days in `month` (1-12) of `year`.
pub const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Number of days in `year` (365 for a common year, 366 for a leap year).
pub const fn days_in_year(year: u16) -> u16 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

/// Milliseconds since the Unix epoch (UTC) of a civil date and time, the
/// inverse of [`DateTime::from_epoch_ms`]. Used to write the wall clock back
/// when a time-setting face (the calendar's settings screen) exits.
pub fn epoch_ms_of(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> u64 {
    let days = days_from_civil(year as i64, month, day);
    (days as u64 * 86_400 + hour as u64 * 3600 + minute as u64 * 60 + second as u64) * 1000
}

/// Day of the week of a civil date (the inverse of [`DateTime::from_epoch_ms`]
/// weekday derivation, via Howard Hinnant's `days_from_civil`). `month` is
/// 1-12, `day` 1-31.
pub fn weekday_of(year: u16, month: u8, day: u8) -> Weekday {
    let days = days_from_civil(year as i64, month, day);
    // 1970-01-01 was a Thursday (4 in the 0=Sunday..6=Saturday scheme).
    Weekday::from_index((days + 4).rem_euclid(7) as u8)
}

/// Days since the Unix epoch (1970-01-01) of a civil date, the inverse of
/// `civil_from_days`. Valid for positive and negative day counts.
fn days_from_civil(y: i64, m: u8, d: u8) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12; // March = 0 .. February = 11
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch() {
        let t = DateTime::from_epoch_ms(0);
        assert_eq!(
            (t.year, t.month, t.day),
            (1970, Month::January, 1)
        );
        assert_eq!((t.hour, t.minute, t.second), (0, 0, 0));
        assert_eq!(t.weekday, Weekday::Thursday);
    }

    #[test]
    fn recent() {
        // 2024-02-29 12:34:56 UTC (leap day)
        let ms = 1_709_210_096_000;
        let t = DateTime::from_epoch_ms(ms);
        assert_eq!(
            (t.year, t.month, t.day),
            (2024, Month::February, 29)
        );
        assert_eq!((t.hour, t.minute, t.second), (12, 34, 56));
        assert_eq!(t.weekday, Weekday::Thursday);
    }

    #[test]
    fn leap_years() {
        assert!(!is_leap_year(2023));
        assert!(is_leap_year(2024));
        assert!(is_leap_year(2000)); // divisible by 400
        assert!(!is_leap_year(2100)); // divisible by 100, not 400
    }

    #[test]
    fn days_in_month_values() {
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
        assert_eq!(days_in_month(2024, 4), 30);
        assert_eq!(days_in_month(2024, 6), 30);
        assert_eq!(days_in_month(2024, 9), 30);
        assert_eq!(days_in_month(2024, 11), 30);
        assert_eq!(days_in_month(2024, 12), 31);
    }

    #[test]
    fn days_in_year_values() {
        assert_eq!(days_in_year(2023), 365);
        assert_eq!(days_in_year(2024), 366);
    }

    #[test]
    fn weekday_of_known_dates() {
        // 1970-01-01 was a Thursday, 2024-02-29 a Thursday (leap day).
        assert_eq!(weekday_of(1970, 1, 1), Weekday::Thursday);
        assert_eq!(weekday_of(2024, 2, 29), Weekday::Thursday);
        // 2026-08-10 is a Monday, 2026-08-07 a Friday, 2001-02-28 a Wednesday.
        assert_eq!(weekday_of(2026, 8, 10), Weekday::Monday);
        assert_eq!(weekday_of(2026, 8, 7), Weekday::Friday);
        assert_eq!(weekday_of(2001, 2, 28), Weekday::Wednesday);
    }

    #[test]
    fn weekday_of_matches_naive() {
        // Cross-check weekday_of against a naive day counter for the calendar
        // face's whole 2000..=2099 range, a few dates per year.
        for year in 2000..=2099u16 {
            for (month, day) in [(1, 1), (2, 28), (3, 1), (12, 31)] {
                let day = day.min(days_in_month(year, month));
                assert_eq!(
                    weekday_of(year, month, day),
                    Weekday::from_index((civil_days_naive(year as i64, month as i64, day as i64) + 4).rem_euclid(7) as u8),
                    "{year}-{month}-{day}"
                );
            }
        }
    }

    #[test]
    fn epoch_ms_of_round_trips() {
        // 2024-02-29 12:34:56 UTC (matches the `recent` test).
        let ms = epoch_ms_of(2024, 2, 29, 12, 34, 56);
        assert_eq!(ms, 1_709_210_096_000);
        let t = DateTime::from_epoch_ms(ms);
        assert_eq!(
            (t.year, t.month, t.day, t.hour, t.minute, t.second),
            (2024, Month::February, 29, 12, 34, 56)
        );
        // The epoch itself.
        assert_eq!(epoch_ms_of(1970, 1, 1, 0, 0, 0), 0);
        // A date/time from the calendar's 2000..=2099 range.
        let t = DateTime::from_epoch_ms(epoch_ms_of(2001, 2, 28, 23, 59, 59));
        assert_eq!(
            (t.year, t.month, t.day, t.hour, t.minute, t.second),
            (2001, Month::February, 28, 23, 59, 59)
        );
    }

    fn civil_days_naive(y: i64, m: i64, d: i64) -> i64 {
        let mut days = (y - 1970) * 365 + (y - 1969) / 4 - (y - 1901) / 100 + (y - 1601) / 400;
        let mdays = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        for i in 1..m {
            days += mdays[i as usize];
        }
        if is_leap_year(y as u16) && m > 2 {
            days += 1;
        }
        days + d - 1
    }
}
