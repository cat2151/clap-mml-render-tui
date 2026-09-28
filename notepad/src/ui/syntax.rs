//! notepad の行のシンタックスハイライト。
//!
//! 行は `{"Surge XT patch": "Pads/Pad 1.fxp", ...} cde` の形。行頭 JSON のうち
//! 音色名（stem とそれ以外）と effect の preset 名、MML 本体の音符の音名に色を付ける。
//! MML 本体のどこが音符かはライブラリの parser に聞く。

use std::ops::Range;

use cmrt_core::EFFECT_CHAIN_JSON_KEY;
use cmrt_patch_select::auto_reverb::AUTO_REVERB_JSON_KEY;
use cmrt_tui_core::theme::{MONOKAI_GREEN, MONOKAI_PINK, MONOKAI_PURPLE, MONOKAI_YELLOW};
use mmlabc_to_smf::{mml_preprocessor, pass1_parser};
use ratatui::style::Color;

use crate::PATCH_JSON_KEY;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyntaxKind {
    /// 音色名の stem 以外（引用符・ディレクトリ・拡張子）。
    PatchPath,
    PatchStem,
    EffectPreset,
    /// MML 本体の音符の `cdefgab`。
    NoteName,
}

impl SyntaxKind {
    pub(crate) fn color(self) -> Color {
        match self {
            Self::PatchPath => MONOKAI_YELLOW,
            Self::PatchStem => MONOKAI_GREEN,
            Self::EffectPreset => MONOKAI_PURPLE,
            Self::NoteName => MONOKAI_PINK,
        }
    }
}

/// 行を色の区切りで分ける。区切りは重ならず、行全体を順に覆う。
pub(crate) fn highlight(line: &str) -> Vec<(Range<usize>, Option<SyntaxKind>)> {
    let preprocessed = mml_preprocessor::extract_embedded_json(line);
    let body_start = if preprocessed.embedded_json.is_some() {
        preprocessed.remaining_offset
    } else {
        0
    };
    let mut marks = json_marks(&line[..body_start]);
    marks.extend(note_marks(&line[body_start..], body_start));
    fill_gaps(line.len(), marks)
}

/// JSON の文字列 literal を順に読み、音色名と effect の preset 名に印を付ける。
fn json_marks(json: &str) -> Vec<(Range<usize>, SyntaxKind)> {
    let mut marks = Vec::new();
    let mut depth = 0usize;
    let mut expecting_value = false;
    let mut top_key: Option<String> = None;
    let mut last_string: Option<String> = None;
    let mut chars = json.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '{' | '[' => {
                depth += 1;
                expecting_value = false;
            }
            '}' | ']' => depth = depth.saturating_sub(1),
            ':' => {
                if depth == 1 {
                    top_key = last_string.take();
                }
                expecting_value = true;
            }
            ',' => expecting_value = false,
            '"' => {
                let mut end = json.len();
                let mut escaped = false;
                for (j, c) in chars.by_ref() {
                    match (escaped, c) {
                        (true, _) => escaped = false,
                        (false, '\\') => escaped = true,
                        (false, '"') => {
                            end = j + 1;
                            break;
                        }
                        _ => {}
                    }
                }
                let literal = i..end;
                if expecting_value {
                    let key = top_key.as_deref();
                    if depth == 1 && key == Some(PATCH_JSON_KEY) {
                        marks.extend(patch_name_marks(json, literal));
                    } else if depth > 1
                        && (key == Some(EFFECT_CHAIN_JSON_KEY) || key == Some(AUTO_REVERB_JSON_KEY))
                    {
                        marks.push((literal, SyntaxKind::EffectPreset));
                    }
                    expecting_value = false;
                } else {
                    last_string = Some(json[literal].trim_matches('"').to_string());
                }
            }
            _ => {}
        }
    }
    marks
}

/// 引用符込みの音色名 literal を、stem とそれ以外に分ける。
fn patch_name_marks(json: &str, literal: Range<usize>) -> Vec<(Range<usize>, SyntaxKind)> {
    let text = &json[literal.clone()];
    let inner_start = literal.start + usize::from(text.starts_with('"'));
    let inner_end = literal.end - usize::from(text.len() > 1 && text.ends_with('"'));
    let inner = &json[inner_start..inner_end];
    let stem_start = inner.rfind(['/', '\\']).map_or(0, |i| i + 1);
    let stem_end = inner[stem_start..]
        .rfind('.')
        .filter(|&i| i > 0)
        .map_or(inner.len(), |i| stem_start + i);
    let stem = inner_start + stem_start..inner_start + stem_end;
    [
        (literal.start..stem.start, SyntaxKind::PatchPath),
        (stem.clone(), SyntaxKind::PatchStem),
        (stem.end..literal.end, SyntaxKind::PatchPath),
    ]
    .into_iter()
    .filter(|(range, _)| !range.is_empty())
    .collect()
}

/// MML 本体の音符の音名。`offset` は行の中での本体の開始位置。
fn note_marks(body: &str, offset: usize) -> Vec<(Range<usize>, SyntaxKind)> {
    let mut marks: Vec<(Range<usize>, SyntaxKind)> = Vec::new();
    for spanned in pass1_parser::parse_mml_spanned(body) {
        if spanned.token.token_type != "note" {
            continue;
        }
        let Some(range) = spanned.byte_range else {
            continue;
        };
        for (i, c) in body[range.clone()].char_indices() {
            let at = offset + range.start + i;
            if matches!(c, 'a'..='g' | 'A'..='G') && !marks.iter().any(|(mark, _)| mark.start == at)
            {
                marks.push((at..at + 1, SyntaxKind::NoteName));
            }
        }
    }
    marks
}

fn fill_gaps(
    len: usize,
    mut marks: Vec<(Range<usize>, SyntaxKind)>,
) -> Vec<(Range<usize>, Option<SyntaxKind>)> {
    marks.sort_by_key(|(range, _)| range.start);
    let mut segments = Vec::new();
    let mut pos = 0;
    for (range, kind) in marks {
        if range.start < pos || range.end > len {
            continue;
        }
        if pos < range.start {
            segments.push((pos..range.start, None));
        }
        pos = range.end;
        segments.push((range, Some(kind)));
    }
    if pos < len {
        segments.push((pos..len, None));
    }
    segments
}

#[cfg(test)]
mod tests;
