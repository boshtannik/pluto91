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
}
