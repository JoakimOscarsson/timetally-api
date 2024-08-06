//! # Work Hours Calculator
//!
//! This module provides functionality for calculating work hours between two dates,
//! taking into account weekends and Swedish holidays.
//!
//! ## Key Concepts
//!
//! - **Reporting Period**: A span of time, typically a week, for which work hours are calculated.
//! - **Work Hours**: The number of working hours in a period, excluding weekends and holidays.

use chrono::{Datelike, Duration, NaiveDate};
use serde::Serialize;
use std::{cmp, collections::BTreeMap};

/// Extends NaiveDate with additional functionality
trait NaiveDateExt {
    /// Returns the number of days in the month for this date
    fn days_in_month(&self) -> i32;
    /// Determines if the year of this date is a leap year
    fn is_leap_year(&self) -> bool;
}
impl NaiveDateExt for chrono::NaiveDate {
    fn days_in_month(&self) -> i32 {
        let month = self.month();
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if self.is_leap_year() {
                    29
                } else {
                    28
                }
            }
            _ => panic!("Invalid month: {}", month),
        }
    }

    fn is_leap_year(&self) -> bool {
        let year = self.year();
        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }
}

/// Represents the calculated work hours for a given time range
#[derive(Serialize, Debug)]
pub struct WorkHours {
    /// Nested structure of years, months, and weeks with their respective work hours
    #[serde(flatten)]
    years: BTreeMap<String, Year>,
    /// Total work hours across all periods
    total: u32,
}

/// Represents work hours for a specific year
#[derive(Serialize, Debug)]
struct Year {
    /// Months in the year with their work hours
    #[serde(flatten)]
    months: BTreeMap<String, Month>,
    /// Total work hours for the year
    total: u32,
}

/// Represents work hours for a specific month
#[derive(Serialize, Debug)]
struct Month {
    /// Work periods in the month with their work hours
    #[serde(flatten)]
    weeks: BTreeMap<String, u32>,
    /// Total work hours for the month
    total: u32,
}

/// Calculates work hours for a period between two dates (inclusive)
///
/// # Arguments
///
/// * `start` - Start date in the format "DD-MM-YYYY"
/// * `end` - End date in the format "DD-MM-YYYY"
///
/// # Returns
///
/// A `Result` containing a `WorkHours` struct if successful, or an error message if not
///
/// # Errors
///
/// Returns an error if:
/// - The date strings are not in the correct format
/// - The start date is after the end date
///
/// # Example
///
/// ```
/// use time_tally::workhours::calculate_workhours;
/// use axum::response::Json;
///
/// let work_hours = Json(calculate_workhours("01-01-2024".to_string(), "31-12-2024".to_string()).unwrap());
/// println!("Total work hours in 2024: {:#?}", work_hours);
/// ```
pub fn calculate_workhours(start: String, end: String) -> Result<WorkHours, String> {
    //Convert to dates
    let (start_date, end_date) = parse_dates(start, end)?;

    let mut years: BTreeMap<String, Year> = BTreeMap::new();
    let mut total_workhours = 0;

    let mut current_date = start_date;
    while current_date <= end_date {
        //Make keys
        let year = current_date.year().to_string();
        let month = format!("{:02}-{}", current_date.month(), current_date.format("%B"));

        //Calculate workhours in current week
        let (week, workhours, period_end_date) = calculate_period(&current_date, &end_date)?;

        //check if year is in years and add it if not
        let year_entry = years.entry(year).or_insert_with(|| Year {
            months: BTreeMap::new(),
            total: 0,
        });

        //check if month is in year.months and add it if not
        let month_entry = year_entry.months.entry(month).or_insert_with(|| Month {
            weeks: BTreeMap::new(),
            total: 0,
        });

        //Add current week to year.month
        month_entry.weeks.insert(week, workhours);

        //Aggregate workhour sums
        total_workhours += workhours;
        year_entry.total += workhours;
        month_entry.total += workhours;

        current_date = period_end_date + Duration::days(1);
    }

    //Return
    Ok(WorkHours {
        years,
        total: total_workhours,
    })
}

/// Parses date strings into NaiveDate objects
///
/// # Arguments
///
/// * `start` - Start date string in the format "DD-MM-YYYY"
/// * `end` - End date string in the format "DD-MM-YYYY"
///
/// # Returns
///
/// A `Result` containing a tuple of `(NaiveDate, NaiveDate)` if successful, or an error message if not
///
/// # Errors
///
/// Returns an error if:
/// - The date strings are not in the correct format
/// - The start date is after the end date
fn parse_dates(start: String, end: String) -> Result<(NaiveDate, NaiveDate), String> {
    // TODO: include more date format checks from the explore project
    let start_date =
        NaiveDate::parse_from_str(&start, "%d-%m-%Y").map_err(|_| "Invalid start date")?;

    let end_date = NaiveDate::parse_from_str(&end, "%d-%m-%Y").map_err(|_| "Invalid start date")?;

    if start_date > end_date {
        return Err("Start date must be before end date".to_string());
    }
    Ok((start_date, end_date))
}

/// Calculates work hours for a specific period
///
/// # Arguments
///
/// * `start_date` - The start date of the period
///
/// # Returns
///
/// A `Result` containing a tuple of `(String, u32, NaiveDate)` representing:
/// - The period name (e.g., "week: 23")
/// - The number of work hours in the period
/// - The end date of the period
///
/// # Errors
///
/// Returns an error if there's an issue calculating holidays
fn calculate_period(
    start_date: &NaiveDate,
    end_date: &NaiveDate,
) -> Result<(String, u32, NaiveDate), String> {
    let mut hours = 0;
    let mut date = *start_date;

    let (period_start, period_end) = period_boundaries(start_date)?;
    let period_name = period_name(&period_start, &period_end);
    let holidays = holidays::for_years(start_date.year(), period_end.year())?;

    let period_end = *cmp::min(end_date, &period_end);
    while date <= period_end {
        hours += if date.weekday() == chrono::Weekday::Sat
            || date.weekday() == chrono::Weekday::Sun
            || holidays.contains(&date)
        {
            0
        } else {
            8
        };

        date += Duration::days(1);
    }
    Ok((period_name, hours, period_end))
}

/// Determines the boundaries of a reporting period for a given date
///
/// # Arguments
///
/// * `date` - A date within the reporting period
///
/// # Returns
///
/// A `Result` containing a tuple of `(NaiveDate, NaiveDate)` representing the start and end dates of the period
///
/// # Errors
///
/// Returns an error if the calculation results in an invalid date
fn period_boundaries(date: &NaiveDate) -> Result<(NaiveDate, NaiveDate), String> {
    let latest_monday = (date.day() as i8) - (date.weekday().num_days_from_monday() as i8);
    let last_week_len = (date.days_in_month() as i8) - latest_monday;

    let (start_date, len) = match latest_monday {
        -5..=-2 => (
            NaiveDate::from_ymd_opt(date.year(), date.month(), 1).ok_or("Invalid start date")?,
            latest_monday + 13,
        ),
        -1..=5 => (
            NaiveDate::from_ymd_opt(date.year(), date.month(), 1).ok_or("Invalid start date")?,
            latest_monday + 6,
        ),
        6..=31 => {
            let new_day = if last_week_len > 1 {
                date.day() - date.weekday().num_days_from_monday()
            } else {
                date.day() - date.weekday().num_days_from_monday() - 7
            };
            let new_date = NaiveDate::from_ymd_opt(date.year(), date.month(), new_day)
                .ok_or("Invalid calculated date")?;

            let len = match last_week_len {
                0 | 1 => last_week_len + 8,
                2..=8 => last_week_len + 1,
                9..=31 => 7,
                _ => return Err("Resulting calc should never be outside 0..=31.".into()),
            };
            (new_date, len)
        }
        _ => return Err("Resulting calc should never be outside -5..=31.".into()),
    };
    let end_date = start_date + Duration::days(len as i64 - 1);

    Ok((start_date, end_date))
}

/// Generates a name for a reporting period
///
/// # Arguments
///
/// * `start` - The start date of the period
/// * `end` - The end date of the period
///
/// # Returns
///
/// A `String` representing the period name (e.g., "week: 23")
fn period_name(start: &NaiveDate, end: &NaiveDate) -> String {
    let len = (*end - *start).num_days();
    if start.weekday() == chrono::Weekday::Mon && len >= 7 {
        format!("week: {}", start.iso_week().week())
    } else {
        format!("week: {}", end.iso_week().week())
    }
}

/// Module for handling Swedish holidays
mod holidays {
    use chrono::{Datelike, Duration, NaiveDate};
    /// Calculates holidays for a range of years
    ///
    /// # Arguments
    ///
    /// * `start_year` - The first year to calculate holidays for
    /// * `end_year` - The last year to calculate holidays for
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<NaiveDate>` of all holidays in the given range, or an error message
    ///
    /// # Errors
    ///
    /// Returns an error if there's an issue calculating holidays for any year in the range
    pub fn for_years(start_year: i32, end_year: i32) -> Result<Vec<NaiveDate>, String> {
        let mut holidays: Vec<NaiveDate> = Vec::new();
        for year in start_year..=end_year {
            let year_holidays = get_year_holidays(year)?;
            holidays.extend(year_holidays.iter().cloned());
        }
        Ok(holidays)
    }

    ///Returns a list of fixed holiday days in Sweden
    /// Gets holidays for a specific year
    ///
    /// # Arguments
    ///
    /// * `year` - The year to calculate holidays for
    ///
    /// # Returns
    ///
    /// A `Result` containing an array of 12 `NaiveDate` objects representing the holidays for the year, or an error message
    ///
    /// # Errors
    ///
    /// Returns an error if there's an issue calculating any of the holidays
    fn get_year_holidays(year: i32) -> Result<[NaiveDate; 12], String> {
        let fixed_dates = [
            NaiveDate::from_ymd_opt(year, 1, 1).ok_or("Failed to initiate fixed date")?,
            NaiveDate::from_ymd_opt(year, 1, 6).ok_or("Failed to initiate fixed date")?,
            NaiveDate::from_ymd_opt(year, 5, 1).ok_or("Failed to initiate fixed date")?,
            NaiveDate::from_ymd_opt(year, 12, 24).ok_or("Failed to initiate fixed date")?,
            NaiveDate::from_ymd_opt(year, 12, 25).ok_or("Failed to initiate fixed date")?,
            NaiveDate::from_ymd_opt(year, 12, 26).ok_or("Failed to initiate fixed date")?,
            NaiveDate::from_ymd_opt(year, 12, 31).ok_or("Failed to initiate fixed date")?,
        ];
        let easter_dates = easter(year)?;

        Ok([
            fixed_dates[0],
            fixed_dates[1],
            fixed_dates[2],
            fixed_dates[3],
            fixed_dates[4],
            fixed_dates[5],
            fixed_dates[6],
            easter_dates[0],
            easter_dates[1],
            easter_dates[2],
            midsummer(year)?,
            national_day(year)?,
        ])
    }

    ///Returns the friday before easter, monday after easter and ascension date.
    fn easter(year: i32) -> Result<[NaiveDate; 3], String> {
        let easter = computus::gregorian_naive(year)?;
        Ok([
            easter - Duration::days(2),  //Långfredag
            easter + Duration::days(1),  //Annandag
            easter + Duration::days(40), //Kristihimmelsfärd
        ])
    }

    ///Returns the Swedish naitonal day if it is not on a weekend. Otherwise, returns the friday before.
    fn national_day(year: i32) -> Result<NaiveDate, String> {
        let national_day =
            NaiveDate::from_ymd_opt(year, 6, 6).ok_or("Failed to calculate the national day")?;
        match national_day.weekday() {
            chrono::Weekday::Sat => Ok(national_day - Duration::days(1)),
            chrono::Weekday::Sun => Ok(national_day - Duration::days(2)),
            _ => Ok(national_day),
        }
    }

    ///Calculates date of Swedish midsummer given a year.
    fn midsummer(year: i32) -> Result<NaiveDate, String> {
        let mut date = NaiveDate::from_ymd_opt(year, 6, 30)
            .ok_or("Failed when initiating midsummer date calculation")?;
        while date.weekday().num_days_from_monday() != 4 {
            date = date
                .pred_opt()
                .ok_or("Failed when stepping dates towards midsummer")?;
        }
        Ok(date)
    }
}

#[cfg(test)]
mod enddate_tests {
    use super::*;
    //5-11 aug 2024 -> +4: (thu-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
    #[test]
    fn early_period_4_1() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 8, 11).unwrap();

        for day in 5..=11 {
            //TODO: Perhaps possible to parameterize into individual cases using macros?
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 8, day).unwrap()).unwrap();

            assert_eq!(last_day, last_day_ref);
        }
    }

    //1-4 aug 2024  -> +4: (thu-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
    #[test]
    fn early_period_4_2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 8, 11).unwrap();

        for day in 1..=4 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 8, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }

    //4-10 nov 2024 -> +3: (fri-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    #[test]
    fn early_period_3_1() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 11, 10).unwrap();

        for day in 4..=10 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 11, day).unwrap()).unwrap();

            assert_eq!(last_day, last_day_ref);
        }
    }
    //1-3 nov 2024  -> +3: (fri-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    #[test]
    fn early_period_3_2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 11, 10).unwrap();

        for day in 1..=3 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 11, day).unwrap()).unwrap();

            assert_eq!(last_day, last_day_ref);
        }
    }
    //3-9 jun 2024  -> +2: (sat-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9]
    #[test]
    fn early_period_2_1() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 6, 9).unwrap();

        for day in 3..=9 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 6, day).unwrap()).unwrap();

            assert_eq!(last_day, last_day_ref);
        }
    }
    //1-2 jun 2024  -> +2: (sat-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9]
    #[test]
    fn early_period_2_2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 6, 9).unwrap();

        for day in 1..=2 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 6, day).unwrap()).unwrap();

            assert_eq!(last_day, last_day_ref);
        }
    }
    //2-8 sep 2024  -> +1: (sun-sun): [1, 2, 3, 4, 5, 6, 7, 8]
    #[test]
    fn early_period_1_1() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 9, 8).unwrap();

        for day in 2..=8 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 9, day).unwrap()).unwrap();

            assert_eq!(last_day, last_day_ref);
        }
    }
    //1 sep 2024    -> +1: (sun-sun): [1, 2, 3, 4, 5, 6, 7, 8]
    #[test]
    fn early_period_1_2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 9, 8).unwrap();

        let (_, last_day) =
            period_boundaries(&NaiveDate::from_ymd_opt(2024, 9, 1).unwrap()).unwrap();

        assert_eq!(last_day, last_day_ref);
    }
    //1-7 apr 2024  -> +0: (mon-sun): [1, 2, 3, 4, 5, 6, 7]
    #[test]
    fn early_period_0() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 4, 7).unwrap();

        for day in 1..=7 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 4, day).unwrap()).unwrap();

            assert_eq!(last_day, last_day_ref);
        }
    }
    //1-6 okt 2024  -> -1: (tue-sun): [1, 2, 3, 4, 5, 6]
    #[test]
    fn early_period_neg1() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 10, 6).unwrap();

        for day in 1..=6 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 10, day).unwrap()).unwrap();

            assert_eq!(last_day, last_day_ref);
        }
    }
    //1-5 may 2024  -> -2: (wed-sun): [1, 2, 3 , 4, 5]
    #[test]
    fn early_period_neg2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 5, 5).unwrap();

        for day in 1..=5 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 5, day).unwrap()).unwrap();

            assert_eq!(last_day, last_day_ref);
        }
    }
    //1-4 apr 2027  -> +4: (thu-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]   26->+1v
    #[test]
    fn early_period_4_1_2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2027, 4, 11).unwrap();

        for day in 1..=4 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2027, 4, day).unwrap()).unwrap();

            assert_eq!(last_day, last_day_ref);
        }
    }
    //5-11 apr 2027 -> +4: (thu-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]   29->normal
    #[test]
    fn early_period_4_2_2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2027, 4, 11).unwrap();

        for day in 5..=11 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2027, 4, day).unwrap()).unwrap();

            assert_eq!(last_day, last_day_ref);
        }
    }
    //-------------------------------------------------------
    //Test cases for middle of month
    //6-12 may 2024 -> +0: (mon-sun): [6, 7, 8, 9, 10, 11, 12]
    #[test]
    fn mid_period() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 5, 12).unwrap();

        for day in 6..=12 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 5, day).unwrap()).unwrap();

            assert_eq!(last_day, last_day_ref);
        }
    }
    //8-14 apr 2024 -> +0: (mon-sun): [8, 9, 19, 11, 12, 13, 14]            16->normal
    #[test]
    fn mid_period_2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 4, 14).unwrap();

        for day in 8..=14 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 4, day).unwrap()).unwrap();

            assert_eq!(last_day, last_day_ref);
        }
    }
    //27 oct 2024   -> +0:                                                  4->normal
    #[test]
    fn mid_period_3() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 10, 27).unwrap();
        let (_, last_day) =
            period_boundaries(&NaiveDate::from_ymd_opt(2024, 10, 27).unwrap()).unwrap();
        assert_eq!(last_day, last_day_ref);
    }
    //28 jul 2024   -> +0:                                                  3->normal
    #[test]
    fn mid_period_4() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 7, 28).unwrap();
        let (_, last_day) =
            period_boundaries(&NaiveDate::from_ymd_opt(2024, 7, 28).unwrap()).unwrap();
        assert_eq!(last_day, last_day_ref);
    }
    //27 apr 2025   -> +0:                                                  3->normal
    #[test]
    fn mid_period_5() {
        let last_day_ref = NaiveDate::from_ymd_opt(2025, 4, 27).unwrap();
        let (_, last_day) =
            period_boundaries(&NaiveDate::from_ymd_opt(2025, 4, 27).unwrap()).unwrap();
        assert_eq!(last_day, last_day_ref);
    }
    //21    apr 2024
    #[test]
    fn mid_period_6() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 4, 21).unwrap();
        let (_, last_day) =
            period_boundaries(&NaiveDate::from_ymd_opt(2024, 4, 21).unwrap()).unwrap();
        assert_eq!(last_day, last_day_ref);
    }
    //-------------------------------------------------------
    //Test cases for end of month
    //22-28 apr 2024 -> +2 (mon-tue) [22, 23, 24, 25, 26, 27, 28, 29, 30]
    #[test]
    fn late_period_2_1() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 4, 30).unwrap();

        for day in 22..=28 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 4, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //29-30 apr 2024 -> +2 (mon-tue) [22, 23, 24, 25, 26, 27, 28, 29, 30]
    #[test]
    fn late_period_2_2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 4, 30).unwrap();

        for day in 29..=30 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 4, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //23-29 sep 2024 -> +1 (mon-mon) [23, 24, 25, 26, 27, 28, 29, 30]
    #[test]
    fn late_period_1_1() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 9, 30).unwrap();

        for day in 23..=29 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 9, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //30 sep 2024    -> +1 (mon-mon) [23, 24, 25, 26, 27, 28, 29, 30]
    #[test]
    fn late_period_1_2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 9, 30).unwrap();

        let (_, last_day) =
            period_boundaries(&NaiveDate::from_ymd_opt(2024, 9, 30).unwrap()).unwrap();
        assert_eq!(last_day, last_day_ref);
    }
    //25-31 mar 2024 -> +0 (mon-sun) [25, 26, 27, 28, 29, 30, 31]
    #[test]
    fn late_period_0() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 3, 31).unwrap();

        for day in 25..=31 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 3, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //26-31 aug 2024 -> -1 (mon-sat) [26, 27, 28, 29, 30, 31]
    #[test]
    fn late_period_neg1() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 8, 31).unwrap();

        for day in 26..=31 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 8, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //27-31 may 2024 -> -2 (mon-fri) [27, 28, 29, 30, 31]
    #[test]
    fn late_period_neg2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 5, 31).unwrap();
        for day in 27..=31 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 5, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //28-31 oct 2024 -> -3 (mon-thu) [28, 29, 30, 31]
    #[test]
    fn late_period_neg3() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 10, 31).unwrap();
        for day in 28..=31 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 10, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //29-31 jul 2024 -> -4 (mon-wed) [20, 30, 31]
    #[test]
    fn late_period_neg4() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 7, 31).unwrap();
        for day in 29..=31 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 7, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //23-29 mar 2026 -> +2 (mon-ue) [23, 24, 25, 26, 27, 28, 29, 30, 31]
    #[test]
    fn late_period_2_1_2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        for day in 23..=29 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2026, 3, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //30-31 mar 2026 -> +2 (mon-ue) [23, 24, 25, 26, 27, 28, 29, 30, 31]
    #[test]
    fn late_period_2_2_2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        for day in 30..=31 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2026, 3, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    #[test]
    fn late_period_neg5() {
        let last_day_ref = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
        for day in 28..=30 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2025, 4, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //-------------------------------------------------------
    //Extra tesets for february
    //23-28 feb 2026 -> -1 (mon-sat) [23, 24, 25, 26, 27, 28]               -1->last_day
    #[test]
    fn feb1() {
        let last_day_ref = NaiveDate::from_ymd_opt(2026, 2, 28).unwrap();
        for day in 23..=28 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2026, 2, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //24-28 feb 2025 -> -2 (mon-fri) [24, 25, 26, 27, 28]                   -2->last_day
    #[test]
    fn feb2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2025, 2, 28).unwrap();
        for day in 24..=28 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2025, 2, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //26-29 feb 2024 -> -3 (mon-thu) [26, 27, 28, 29]                       -3->last_day
    #[test]
    fn feb3() {
        let last_day_ref = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        for day in 26..=29 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 2, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //22-28 feb 2027 -> +0 (mon-sun) [22, 23, 24, 25, 26, 27, 28]           0->last_day
    #[test]
    fn feb4() {
        let last_day_ref = NaiveDate::from_ymd_opt(2027, 2, 28).unwrap();
        for day in 22..=28 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2027, 2, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //21-27 feb 2028 -> +2 (mon-tue) [21, 22, 23, 24, 25, 26, 27, 28, 29]   2->last_day
    #[test]
    fn feb5_1() {
        let last_day_ref = NaiveDate::from_ymd_opt(2028, 2, 29).unwrap();
        for day in 21..=27 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2028, 2, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
    //28-29 feb 2028 -> +2 (mon-tue) [21, 22, 23, 24, 25, 26, 27, 28, 29]  -5->last_day
    #[test]
    fn feb5_2() {
        let last_day_ref = NaiveDate::from_ymd_opt(2028, 2, 29).unwrap();
        for day in 28..=29 {
            let (_, last_day) =
                period_boundaries(&NaiveDate::from_ymd_opt(2028, 2, day).unwrap()).unwrap();
            assert_eq!(last_day, last_day_ref);
        }
    }
}
#[cfg(test)]
mod reportperiod_tests {
    use super::*;

    //testing against panic over 2 years
    // #[ignore]
    // #[test]
    // fn test_two_years() {
    //     for d in 0..=1825 {
    //         let date = NaiveDate::from_num_days_from_ce_opt(735671+d).expect("crashed when creating date");
    //         ReportPeriod::new(&date);
    //     }
    // }

    //Testing startdates and lengths
    //Test cases for beginning of month
    //5-11 aug 2024 -> +4: (thu-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
    #[test]
    fn early_period_4_1() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 8, 1).unwrap();
        for day in 5..=11 {
            //TODO: Perhaps possible to parameterize into individual cases using macros?
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 8, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }

    //1-4 aug 2024  -> +4: (thu-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
    #[test]
    fn early_period_4_2() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 8, 1).unwrap();
        for day in 1..=4 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 8, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }

    //4-10 nov 2024 -> +3: (fri-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    #[test]
    fn early_period_3_1() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 11, 1).unwrap();
        for day in 4..=10 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 11, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //1-3 nov 2024  -> +3: (fri-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    #[test]
    fn early_period_3_2() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 11, 1).unwrap();
        for day in 1..=3 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 11, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //3-9 jun 2024  -> +2: (sat-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9]
    #[test]
    fn early_period_2_1() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        for day in 3..=9 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 6, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //1-2 jun 2024  -> +2: (sat-sun): [1, 2, 3, 4, 5, 6, 7, 8, 9]
    #[test]
    fn early_period_2_2() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        for day in 1..=2 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 6, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //2-8 sep 2024  -> +1: (sun-sun): [1, 2, 3, 4, 5, 6, 7, 8]
    #[test]
    fn early_period_1_1() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();
        for day in 2..=8 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 9, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //1 sep 2024    -> +1: (sun-sun): [1, 2, 3, 4, 5, 6, 7, 8]
    #[test]
    fn early_period_1_2() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();
        let (startdate, _) =
            period_boundaries(&NaiveDate::from_ymd_opt(2024, 9, 1).unwrap()).unwrap();
        assert_eq!(startdate, startdate_ref);
    }
    //1-7 apr 2024  -> +0: (mon-sun): [1, 2, 3, 4, 5, 6, 7]
    #[test]
    fn early_period_0() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 4, 1).unwrap();
        for day in 1..=7 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 4, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //1-6 okt 2024  -> -1: (tue-sun): [1, 2, 3, 4, 5, 6]
    #[test]
    fn early_period_neg1() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 10, 1).unwrap();
        for day in 1..=6 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 10, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //1-5 may 2024  -> -2: (wed-sun): [1, 2, 3 , 4, 5]
    #[test]
    fn early_period_neg2() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 5, 1).unwrap();
        for day in 1..=5 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 5, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }

    //-------------------------------------------------------
    //Test cases for middle of month
    //6-12 may 2024 -> +0: (mon-sun): [6, 7, 8, 9, 10, 11, 12]
    #[test]
    fn mid_period() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 5, 6).unwrap();
        for day in 6..=12 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 5, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }

    //-------------------------------------------------------
    //Test cases for end of month
    //22-28 apr 2024 -> +2 (mon-tue) [22, 23, 24, 25, 26, 27, 28, 29, 30]
    #[test]
    fn late_period_2_1() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 4, 22).unwrap();
        for day in 22..=28 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 4, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //29-30 apr 2024 -> +2 (mon-tue) [22, 23, 24, 25, 26, 27, 28, 29, 30]
    #[test]
    fn late_period_2_2() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 4, 22).unwrap();
        for day in 29..=30 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 4, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //23-29 sep 2024 -> +1 (mon-mon) [23, 24, 25, 26, 27, 28, 29, 30]
    #[test]
    fn late_period_1_1() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 9, 23).unwrap();
        for day in 23..=29 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 9, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //30 sep 2024    -> +1 (mon-mon) [23, 24, 25, 26, 27, 28, 29, 30]
    #[test]
    fn late_period_1_2() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 9, 23).unwrap();
        let (startdate, _) =
            period_boundaries(&NaiveDate::from_ymd_opt(2024, 9, 30).unwrap()).unwrap();
        assert_eq!(startdate, startdate_ref);
    }
    //25-31 mar 2024 -> +0 (mon-sun) [25, 26, 27, 28, 29, 30, 31]
    #[test]
    fn late_period_0() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 3, 25).unwrap();
        for day in 25..=31 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 3, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //26-31 aug 2024 -> -1 (mon-sat) [26, 27, 28, 29, 30, 31]
    #[test]
    fn late_period_neg1() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 8, 26).unwrap();
        for day in 26..=31 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 8, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //27-31 may 2024 -> -2 (mon-fri) [27, 28, 29, 30, 31]
    #[test]
    fn late_period_neg2() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 5, 27).unwrap();
        for day in 27..=31 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 5, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //28-31 oct 2024 -> -3 (mon-thu) [28, 29, 30, 31]
    #[test]
    fn late_period_neg3() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 10, 28).unwrap();
        for day in 28..=31 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 10, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
    //29-31 jul 2024 -> -4 (mon-wed) [20, 30, 31]
    #[test]
    fn late_period_neg4() {
        let startdate_ref = NaiveDate::from_ymd_opt(2024, 7, 29).unwrap();
        for day in 29..=31 {
            let (startdate, _) =
                period_boundaries(&NaiveDate::from_ymd_opt(2024, 7, day).unwrap()).unwrap();
            assert_eq!(startdate, startdate_ref);
        }
    }
}
