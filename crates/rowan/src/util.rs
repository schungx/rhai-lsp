use crate::parser::{parsers::parse_expr, Parse, Parser};
use rowan::{TextRange, TextSize};
use thiserror::Error;

pub struct Interpolated<'s> {
    pub segments: Vec<(InterpolatedSegment<'s>, TextRange)>,
}

pub enum InterpolatedSegment<'s> {
    Str(&'s str),
    Interpolation(Parse),
}

#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn parse_interpolated(s: &str) -> Interpolated {
    let mut segments = Vec::new();

    let mut chars = s.char_indices().peekable();
    let mut segment_start = 0usize;
    let mut segment_end = 0usize;

    while let Some((i, ch)) = chars.next() {
        match ch {
            '$' if matches!(chars.peek(), Some((_, '{'))) => {
                if segment_start < i {
                    segments.push((
                        InterpolatedSegment::Str(&s[segment_start..i]),
                        TextRange::new(
                            TextSize::from(segment_start as u32),
                            TextSize::from(i as u32),
                        ),
                    ));
                }

                let (i, _) = chars.next().unwrap();
                segment_start = i;

                let mut parser = Parser::new(&s[i..]);
                parser.execute(parse_expr);
                let parse = parser.finish();

                let end_idx = (i as u32 + u32::from(parse.green.text_len())) as usize;

                for (i, _) in chars.by_ref() {
                    if i >= end_idx {
                        break;
                    }
                }

                segments.push((
                    InterpolatedSegment::Interpolation(parse),
                    TextRange::new(
                        TextSize::from(segment_start as u32),
                        TextSize::from(end_idx as u32),
                    ),
                ));

                if matches!(chars.peek(), Some((_, '}'))) {
                    let (i, _) = chars.next().unwrap();
                    segment_start = i;
                }
            }
            _ => {
                segment_end = i;
            }
        }
    }

    if segment_start < segment_end {
        segments.push((
            InterpolatedSegment::Str(&s[segment_start..segment_end]),
            TextRange::new(
                TextSize::from(segment_start as u32),
                TextSize::from(segment_end as u32),
            ),
        ));
    }

    Interpolated { segments }
}

#[must_use]
pub fn unescape(s: &str, termination_char: char) -> (String, Vec<EscapeError>) {
    let mut chars = s.chars().peekable();
    let mut result = String::with_capacity(12);
    let mut escape = String::with_capacity(12);
    let mut errors = Vec::new();

    let mut position = TextSize::default();

    while let Some(ch) = chars.next() {
        match ch {
            // \r - ignore if followed by \n
            '\r' if chars.peek().map_or(false, |ch| *ch == '\n') => (),
            // \...
            '\\' if escape.is_empty() => {
                escape.push('\\');
            }
            // \\
            '\\' if !escape.is_empty() => {
                escape.clear();
                result.push('\\');
            }
            // \t
            't' if !escape.is_empty() => {
                escape.clear();
                result.push('\t');
            }
            // \n
            'n' if !escape.is_empty() => {
                escape.clear();
                result.push('\n');
            }
            // \r
            'r' if !escape.is_empty() => {
                escape.clear();
                result.push('\r');
            }
            // \x??, \u????, \U????????
            ch @ ('x' | 'u' | 'U') if !escape.is_empty() => {
                let mut seq = escape.clone();
                escape.clear();
                seq.push(ch);

                let mut out_val: u32 = 0;
                let len = match ch {
                    'x' => 2,
                    'u' => 4,
                    'U' => 8,
                    _ => unreachable!(),
                };

                let mut err = false;
                for _ in 0..len {
                    let c = match chars.next() {
                        Some(ch) => ch,
                        None => {
                            errors.push(EscapeError::MalformedEscapeSequence(
                                seq.clone(),
                                TextRange::new(
                                    position,
                                    position + TextSize::from(escape.len() as u32),
                                ),
                            ));
                            break;
                        }
                    };

                    seq.push(c);

                    out_val *= 16;

                    match c.to_digit(16) {
                        Some(c) => out_val += c,
                        None => {
                            err = true;
                            errors.push(EscapeError::MalformedEscapeSequence(
                                seq.clone(),
                                TextRange::new(
                                    position,
                                    position + TextSize::from(escape.len() as u32),
                                ),
                            ));
                        }
                    }

                    position += TextSize::from(c.len_utf8() as u32);
                }

                if !err {
                    match char::from_u32(out_val) {
                        Some(c) => result.push(c),
                        None => errors.push(EscapeError::MalformedEscapeSequence(
                            seq,
                            TextRange::new(
                                position,
                                position + TextSize::from(escape.len() as u32),
                            ),
                        )),
                    };
                }
            }

            // \{termination_char} - escaped
            _ if termination_char == ch && !escape.is_empty() => {
                escape.clear();
                result.push(ch);
            }

            // Line continuation
            '\n' if !escape.is_empty() => {
                escape.clear();
            }

            // Unknown escape sequence
            _ if !escape.is_empty() => {
                escape.push(ch);
                errors.push(EscapeError::MalformedEscapeSequence(
                    escape.clone(),
                    TextRange::new(position, position + TextSize::from(escape.len() as u32)),
                ));
            }

            // All other characters
            _ => {
                escape.clear();
                result.push(ch);
            }
        }
        position += TextSize::from(ch.len_utf8() as u32);
    }

    (result, errors)
}

#[derive(Debug, Error)]
pub enum EscapeError {
    #[error("malformed escape sequence `{0}`")]
    MalformedEscapeSequence(String, TextRange),
}
