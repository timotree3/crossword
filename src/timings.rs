//! # NY Crossword Times
//!
//! Provides timing related to NYTimes Crosswords.
//! For more info, see the [FAQ].
//! [FAQ]: https://www.nytimes.com/content/help/games/crosswords/crosswords.html#available


use std;
use chrono::{DateTime, Datelike, Date, TimeZone, Utc};
use chrono_tz::America::New_York;
use chrono_tz::Tz;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        // let day = New_York.ymd(2017, 10, 11);
        // assert_eq!(night_release_of(&day), day.and_hms(12+10, 0, 0));
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Puzzle {
    date: Date<Tz>,
}

impl Puzzle {
    /// Returns the puzzle associated with `day`. TimeZone must be New_York.
    fn of(day: Date<Tz>) -> Puzzle {
        assert!(day.timezone() == New_York);
        Puzzle { date: day }
    }

    /// Returns the next puzzle.
    pub fn succ(self) -> Puzzle {
        Puzzle::of(self.date.succ())
    }

    // Returns the previous puzzle.
    pub fn pred(self) -> Puzzle {
        Puzzle::of(self.date.pred())
    }

    /// Returns the current puzzle as of `time`.
    pub fn current_as_of<T>(time: DateTime<T>) -> Puzzle
    where
        T: TimeZone,
    {
        let time = time.with_timezone(&New_York);
        let puzzle = Puzzle::of(time.date());
        if time < puzzle.replacement_time() {
            puzzle
        } else {
            puzzle.succ()
        }
    }

    /// Returns the time when this puzzle's replacement will come out.
    pub fn replacement_time(self) -> DateTime<Tz> {
        use chrono::Weekday;
        let hour = match self.date.weekday() {
            Weekday::Sat | Weekday::Sun => 18, // 6PM
            _ => 22, // 10PM
        };
        self.date.and_hms(hour, 0, 0)
    }

    /// Blocks until this puzzle's replacement has been released.
    pub fn wait_until_replaced(self) {
        wait_until(self.replacement_time().with_timezone(&Utc))
    }
}

use std::fmt;

impl fmt::Display for Puzzle {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.date.format("%A, %B %e %Y").fmt(f)
    }
}

/// Blocks until the given time.
pub fn wait_until(stop: DateTime<Utc>) {
    let mut delay = stop.signed_duration_since(Utc::now());
    // delay.to_std() fails if delay is negative
    while let Ok(d) = delay.to_std() {
        std::thread::sleep(d);
        delay = stop.signed_duration_since(Utc::now());
    }
    assert!(stop < Utc::now());
}