use chrono::Weekday;
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use thiserror::Error;

#[derive(Parser)]
#[grammar = "time.pest"]
pub struct TimeParser;

pub type YMD = (i32, u32, u32);
pub type HMS = (u32, u32, u32);

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("invalid integer")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error(transparent)]
    PestError(#[from] pest::error::Error<Rule>),
    #[error("unexpected non matching pattern")]
    UnexpectedNonMatchingPattern,
    #[error("unknown weekday: `{0}`")]
    UnknownWeekday(String),
    #[error("unknown shortcut day: `{0}`")]
    UnknownShortcutDay(String),
    #[error("unknown modifier: `{0}`")]
    UnknownModifier(String),
    #[error("unknown quantifier `{0}`")]
    UnknownQuantifier(String),
}

fn weekday_from(s: &str) -> Result<Weekday, ParseError> {
    match s {
        "monday" => Ok(Weekday::Mon),
        "tuesday" => Ok(Weekday::Tue),
        "wednesday" => Ok(Weekday::Wed),
        "thursday" => Ok(Weekday::Thu),
        "friday" => Ok(Weekday::Fri),
        "saturday" => Ok(Weekday::Sat),
        "sunday" => Ok(Weekday::Sun),
        _ => Err(ParseError::UnknownWeekday(s.to_string())),
    }
}

#[derive(Debug, PartialEq)]
pub enum ShortcutDay {
    Today,
    Yesterday,
}

fn shortcut_day_from(s: &str) -> Result<ShortcutDay, ParseError> {
    match s {
        "today" => Ok(ShortcutDay::Today),
        "yesterday" => Ok(ShortcutDay::Yesterday),
        _ => Err(ParseError::UnknownShortcutDay(s.to_string())),
    }
}

#[derive(Debug, PartialEq)]
pub enum Modifier {
    Last,
    Next,
}

fn modifier_from(s: &str) -> Result<Modifier, ParseError> {
    match s {
        "last" => Ok(Modifier::Last),
        "next" => Ok(Modifier::Next),
        _ => Err(ParseError::UnknownModifier(s.to_string())),
    }
}

#[derive(Debug, PartialEq)]
pub enum Quantifier {
    Min,
    Days,
}

fn quantifier_from(s: &str) -> Result<Quantifier, ParseError> {
    match s {
        "min" => Ok(Quantifier::Min),
        "days" => Ok(Quantifier::Days),
        _ => Err(ParseError::UnknownQuantifier(s.to_string())),
    }
}

#[derive(Debug, PartialEq)]
pub enum TimeClue {
    Now,
    Time(HMS),
    Relative(usize, Quantifier),
    RelativeDayAt(Modifier, Weekday, Option<HMS>),
    SameWeekDayAt(Weekday, Option<HMS>),
    ShortcutDayAt(ShortcutDay, Option<HMS>),
    ISO(YMD, HMS),
}

fn parse_time_hms(rules_and_str: &[(Rule, &str)]) -> Result<TimeClue, ParseError> {
    match rules_and_str {
        [(Rule::hms, h)] => {
            let h: u32 = h.parse()?;
            Ok(TimeClue::Time((h, 0, 0)))
        }
        [(Rule::hms, h), (Rule::hms, m)] => {
            let h: u32 = h.parse()?;
            let m: u32 = m.parse()?;
            Ok(TimeClue::Time((h, m, 0)))
        }
        [(Rule::hms, h), (Rule::hms, m), (Rule::hms, s)] => {
            let h: u32 = h.parse()?;
            let m: u32 = m.parse()?;
            let s: u32 = s.parse()?;
            Ok(TimeClue::Time((h, m, s)))
        }
        _ => Err(ParseError::UnexpectedNonMatchingPattern),
    }
}

fn parse_time_clue(pairs: &[Pair<Rule>]) -> Result<TimeClue, ParseError> {
    let rules_and_str: Vec<(Rule, &str)> = pairs
        .iter()
        .map(|pair| (pair.as_rule(), pair.as_str()))
        .collect();
    match rules_and_str.as_slice() {
        [(Rule::time_clue, _), (Rule::now, _), (Rule::EOI, _)] => Ok(TimeClue::Now),
        [(Rule::time_clue, _), (Rule::time, _), time_hms @ .., (Rule::EOI, _)] => {
            parse_time_hms(time_hms)
        }
        [(Rule::time_clue, _), (Rule::relative, _), (Rule::int, s), (Rule::quantifier, q), (Rule::EOI, _)] =>
        {
            let n: usize = s.parse()?;
            let q = quantifier_from(q)?;
            Ok(TimeClue::Relative(n, q))
        }
        [(Rule::time_clue, _), (Rule::day_at, _), (Rule::mday, _), mday @ .., (Rule::EOI, _)] => {
            match mday {
                [(Rule::modifier, m), (Rule::weekday, w), (Rule::time, _), time_hms @ ..] => {
                    let time_maybe = match parse_time_hms(time_hms)? {
                        TimeClue::Time(hms) => Some(hms),
                        _ => None,
                    };
                    let m = modifier_from(m)?;
                    let w = weekday_from(w)?;
                    Ok(TimeClue::RelativeDayAt(m, w, time_maybe))
                }
                [(Rule::modifier, m), (Rule::weekday, w)] => {
                    let m = modifier_from(m)?;
                    let w = weekday_from(w)?;
                    Ok(TimeClue::RelativeDayAt(m, w, None))
                }
                [(Rule::weekday, w), (Rule::time, _), time_hms @ ..] => {
                    let time_maybe = match parse_time_hms(time_hms)? {
                        TimeClue::Time(hms) => Some(hms),
                        _ => None,
                    };
                    let w = weekday_from(w)?;
                    Ok(TimeClue::SameWeekDayAt(w, time_maybe))
                }
                [(Rule::shortcut_day, r), (Rule::time, _), time_hms @ ..] => {
                    let time_maybe = match parse_time_hms(time_hms)? {
                        TimeClue::Time(hms) => Some(hms),
                        _ => None,
                    };
                    let r = shortcut_day_from(r)?;
                    Ok(TimeClue::ShortcutDayAt(r, time_maybe))
                }
                [(Rule::shortcut_day, r)] => {
                    let r = shortcut_day_from(r)?;
                    Ok(TimeClue::ShortcutDayAt(r, None))
                }
                _ => Err(ParseError::UnexpectedNonMatchingPattern),
            }
        }
        [(Rule::time_clue, _), (Rule::iso, _), (Rule::year, y), (Rule::month, m), (Rule::day, d), (Rule::time, _), time_hms @ .., (Rule::EOI, _)] => {
            match parse_time_hms(time_hms)? {
                TimeClue::Time(hms) => {
                    let y: i32 = y.parse()?;
                    let m: u32 = m.parse()?;
                    let d: u32 = d.parse()?;
                    Ok(TimeClue::ISO((y, m, d), hms))
                }
                _ => Err(ParseError::UnexpectedNonMatchingPattern),
            }
        }
        _ => Err(ParseError::UnexpectedNonMatchingPattern),
    }
}

pub fn parse_time_clue_from_str(s: &str) -> Result<TimeClue, ParseError> {
    let pairs: Pairs<Rule> = TimeParser::parse(Rule::time_clue, s)?;
    let pairs: Vec<Pair<Rule>> = pairs.flatten().collect();
    parse_time_clue(pairs.as_slice())
}

#[cfg(test)]
mod test {
    use crate::parser::{parse_time_clue_from_str, Modifier, Quantifier, ShortcutDay, TimeClue};
    use chrono::Weekday;

    #[test]
    fn test_parse_time_ok() {
        assert_eq!(
            TimeClue::Time((9, 0, 0)),
            parse_time_clue_from_str("9").unwrap()
        );
        assert_eq!(
            TimeClue::Time((9, 30, 0)),
            parse_time_clue_from_str("9:30").unwrap()
        );
        assert_eq!(
            TimeClue::Time((9, 30, 56)),
            parse_time_clue_from_str("9:30:56").unwrap()
        );
    }

    #[test]
    fn test_parse_relative_ok() {
        assert_eq!(
            TimeClue::Relative(2, Quantifier::Min),
            parse_time_clue_from_str("2 min ago").unwrap()
        );
        assert_eq!(
            TimeClue::Relative(2, Quantifier::Min),
            parse_time_clue_from_str("2min ago").unwrap()
        );
        assert_eq!(
            TimeClue::Relative(2, Quantifier::Min),
            parse_time_clue_from_str("2minago").unwrap()
        );
        assert_eq!(
            TimeClue::Relative(2, Quantifier::Min),
            parse_time_clue_from_str("2  min   ago").unwrap()
        );
    }

    #[test]
    fn test_parse_shortcut_day_ok() {
        assert_eq!(
            TimeClue::ShortcutDayAt(ShortcutDay::Today, None),
            parse_time_clue_from_str("today").unwrap()
        );
        assert_eq!(
            TimeClue::ShortcutDayAt(ShortcutDay::Today, Some((7, 0, 0))),
            parse_time_clue_from_str("today at 7").unwrap()
        );
        assert_eq!(
            TimeClue::ShortcutDayAt(ShortcutDay::Yesterday, None),
            parse_time_clue_from_str("yesterday").unwrap()
        );
        assert_eq!(
            TimeClue::ShortcutDayAt(ShortcutDay::Yesterday, Some((19, 43, 0))),
            parse_time_clue_from_str("yesterday at 19:43").unwrap()
        );
        assert_eq!(
            TimeClue::ShortcutDayAt(ShortcutDay::Yesterday, Some((19, 43, 0))),
            parse_time_clue_from_str("yesterday at 19:43:00").unwrap()
        );
    }

    #[test]
    fn test_parse_relative_day_ok() {
        assert_eq!(TimeClue::Now, parse_time_clue_from_str("now").unwrap());
        assert_eq!(
            TimeClue::SameWeekDayAt(Weekday::Fri, Some((19, 43, 0))),
            parse_time_clue_from_str("friday at 19:43").unwrap()
        );
        assert_eq!(
            TimeClue::RelativeDayAt(Modifier::Last, Weekday::Fri, None),
            parse_time_clue_from_str("last friday").unwrap()
        );
        assert_eq!(
            TimeClue::RelativeDayAt(Modifier::Last, Weekday::Fri, Some((9, 0, 0))),
            parse_time_clue_from_str("last friday at 9").unwrap()
        );
    }

    #[test]
    fn test_parse_same_week_ok() {
        assert_eq!(
            TimeClue::SameWeekDayAt(Weekday::Fri, Some((19, 43, 0))),
            parse_time_clue_from_str("friday at 19:43").unwrap()
        );
    }

    #[test]
    fn test_parse_now_ok() {
        assert_eq!(TimeClue::Now, parse_time_clue_from_str("now").unwrap());
    }
}