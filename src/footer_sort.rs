// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use std::cmp::Ordering;

pub fn compare_values(sort_key: &str, left: &str, right: &str) -> Ordering {
    if sort_key == "date" {
        if let (Some(left_date), Some(right_date)) = (parse_date(left), parse_date(right)) {
            return left_date.cmp(&right_date);
        }
    }

    left.cmp(right)
}

pub(crate) fn parse_date(value: &str) -> Option<(u32, u8, u8)> {
    let text = value.trim();
    if text.is_empty() {
        return None;
    }

    parse_numeric_date(text).or_else(|| parse_named_month_date(text))
}

fn parse_numeric_date(text: &str) -> Option<(u32, u8, u8)> {
    let parts: Vec<&str> = text
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 3 {
        return None;
    }

    if parts[0].len() == 4 {
        let year = parts[0].parse::<u32>().ok()?;
        let month = parts[1].parse::<u8>().ok()?;
        let day = parts[2].parse::<u8>().ok()?;
        return validate_ymd(year, month, day);
    }

    if parts[2].len() == 4 {
        let year = parts[2].parse::<u32>().ok()?;
        let first = parts[0].parse::<u8>().ok()?;
        let second = parts[1].parse::<u8>().ok()?;

        // Prefer month/day/year, then try day/month/year.
        return validate_ymd(year, first, second).or_else(|| validate_ymd(year, second, first));
    }

    None
}

fn parse_named_month_date(text: &str) -> Option<(u32, u8, u8)> {
    let normalized = text.replace(',', " ");
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    if let Some(month) = parse_month_token(parts[0]) {
        let day = parse_u8_prefix(parts[1])?;
        let year = parse_u32_prefix(parts[2])?;
        return validate_ymd(year, month, day);
    }

    let day = parse_u8_prefix(parts[0])?;
    let month = parse_month_token(parts[1])?;
    let year = parse_u32_prefix(parts[2])?;
    validate_ymd(year, month, day)
}

fn parse_month_token(token: &str) -> Option<u8> {
    let month = token
        .trim_matches(|ch: char| !ch.is_ascii_alphabetic())
        .to_ascii_lowercase();

    match month.as_str() {
        "january" | "jan" => Some(1),
        "february" | "feb" => Some(2),
        "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "may" => Some(5),
        "june" | "jun" => Some(6),
        "july" | "jul" => Some(7),
        "august" | "aug" => Some(8),
        "september" | "sep" | "sept" => Some(9),
        "october" | "oct" => Some(10),
        "november" | "nov" => Some(11),
        "december" | "dec" => Some(12),
        _ => None,
    }
}

fn parse_u8_prefix(token: &str) -> Option<u8> {
    let len = token.chars().take_while(|ch| ch.is_ascii_digit()).count();
    (len > 0).then(|| token[..len].parse::<u8>().ok()).flatten()
}

fn parse_u32_prefix(token: &str) -> Option<u32> {
    let len = token.chars().take_while(|ch| ch.is_ascii_digit()).count();
    (len > 0)
        .then(|| token[..len].parse::<u32>().ok())
        .flatten()
}

fn validate_ymd(year: u32, month: u8, day: u8) -> Option<(u32, u8, u8)> {
    if month == 0 || month > 12 || day == 0 || day > 31 {
        return None;
    }
    Some((year, month, day))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_values_parses_textual_month_dates() {
        assert_eq!(
            compare_values("date", "August 15, 2021", "January 2, 2020"),
            Ordering::Greater
        );
        assert_eq!(
            compare_values("date", "15 Aug 2021", "2021-08-15"),
            Ordering::Equal
        );
    }

    #[test]
    fn test_compare_values_falls_back_to_string_when_unparseable() {
        assert_eq!(compare_values("date", "unknown", "zzz"), Ordering::Less);
        assert_eq!(compare_values("title", "b", "a"), Ordering::Greater);
    }
}
