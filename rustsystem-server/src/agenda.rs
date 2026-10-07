//! A meeting's agenda, written by hosts in Markdown (`docs/PROTOCOL.md` §4.5).
//!
//! Every heading is an agenda point; the text up to the next heading is that point's body.
//! Parsing uses `pulldown-cmark`, so headings inside code blocks, setext headings and escapes
//! behave exactly as any Markdown viewer shows them.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::Serialize;

use rustsystem_core::{ApiError, ApiResult};

use crate::state::{MAX_LABEL_LENGTH, clean_text};

pub const MAX_AGENDA_POINTS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    /// 1 for `#`, 2 for `##`, and so on.
    pub level: u8,
    pub title: String,
    /// The Markdown between this heading and the next, trimmed. Shown as plain text.
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agenda {
    /// What the host wrote, for editing.
    pub source: String,
    pub points: Vec<Point>,
}

impl Agenda {
    pub fn parse(source: &str) -> ApiResult<Self> {
        // (level, title, byte range of the whole heading)
        let mut headings: Vec<(u8, String, std::ops::Range<usize>)> = Vec::new();
        let mut open: Option<(u8, String, usize)> = None;
        for (event, range) in Parser::new_ext(source, Options::empty()).into_offset_iter() {
            match event {
                Event::Start(Tag::Heading { level, .. }) => open = Some((level_number(level), String::new(), range.start)),
                Event::End(TagEnd::Heading(_)) => {
                    if let Some((level, title, start)) = open.take() {
                        headings.push((level, title, start..range.end));
                    }
                }
                Event::Text(t) | Event::Code(t) => {
                    if let Some((_, title, _)) = open.as_mut() {
                        title.push_str(&t);
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    if let Some((_, title, _)) = open.as_mut() {
                        title.push(' ');
                    }
                }
                _ => {}
            }
        }

        if headings.is_empty() {
            return Err(ApiError::invalid_input(
                "The agenda has no headings. Every heading (# or ##) becomes an agenda point.",
            ));
        }
        if headings.len() > MAX_AGENDA_POINTS {
            return Err(ApiError::invalid_input(format!("An agenda can have at most {MAX_AGENDA_POINTS} points.")));
        }

        let mut points = Vec::with_capacity(headings.len());
        for (i, (level, title, range)) in headings.iter().enumerate() {
            let body_end = headings.get(i + 1).map_or(source.len(), |next| next.2.start);
            points.push(Point {
                level: *level,
                title: clean_text("An agenda heading", title, MAX_LABEL_LENGTH)?,
                body: source[range.end..body_end].trim().to_owned(),
            });
        }
        Ok(Self { source: source.to_owned(), points })
    }
}

fn level_number(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustsystem_core::ErrorCode;

    fn titles(a: &Agenda) -> Vec<(u8, &str)> {
        a.points.iter().map(|p| (p.level, p.title.as_str())).collect()
    }

    #[test]
    fn every_heading_is_a_point_with_its_body() {
        let a = Agenda::parse("Preamble is ignored.\n\n# Opening\nWelcome.\n\n## Election of chair\n\n- Alice\n- Bob\n\n# Closing\n").unwrap();
        assert_eq!(titles(&a), [(1, "Opening"), (2, "Election of chair"), (1, "Closing")]);
        assert_eq!(a.points[0].body, "Welcome.");
        assert_eq!(a.points[1].body, "- Alice\n- Bob");
        assert_eq!(a.points[2].body, "");
    }

    #[test]
    fn setext_headings_and_inline_markup() {
        let a = Agenda::parse("Budget **2027**\n==============\n\nSub `item`\n---\n").unwrap();
        assert_eq!(titles(&a), [(1, "Budget 2027"), (2, "Sub item")]);
    }

    #[test]
    fn hashes_in_code_blocks_are_not_headings() {
        let a = Agenda::parse("# Real\n\n```\n# not a heading\n```\n").unwrap();
        assert_eq!(titles(&a), [(1, "Real")]);
        assert_eq!(a.points[0].body, "```\n# not a heading\n```");
    }

    #[test]
    fn rejects_empty_huge_and_bad_titles() {
        let code = |s: &str| Agenda::parse(s).unwrap_err().code;
        assert_eq!(code("just text"), ErrorCode::InvalidInput);
        assert_eq!(code(&"# x\n".repeat(MAX_AGENDA_POINTS + 1)), ErrorCode::InvalidInput);
        assert_eq!(code(&format!("# {}", "x".repeat(MAX_LABEL_LENGTH + 1))), ErrorCode::InvalidInput);
        assert_eq!(code("#\n"), ErrorCode::InvalidInput, "an empty heading has no title");
        assert!(Agenda::parse(&"# x\n".repeat(MAX_AGENDA_POINTS)).is_ok());
    }
}
