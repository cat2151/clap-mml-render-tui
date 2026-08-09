//! patch 名の正規化・自然順比較・表示名の解決。

use std::cmp::Ordering;

use crate::layout::PATCH_DIR_PREFIXES;

/// 区切り文字と大文字小文字の揺れを吸収した、照合用のキーへ直す。
pub fn normalize_patch_lookup_key(patch_name: &str) -> String {
    patch_name
        .trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_matches('/')
        .to_lowercase()
}

/// `start` 位置から、連続する数字または非数字チャンクの終端位置と、
/// そのチャンクが数字だけで構成されるかどうかを返す。
fn next_chunk(input: &str, start: usize) -> Option<(usize, bool)> {
    let mut chars = input[start..].char_indices();
    let (_, first) = chars.next()?;
    let is_digit = first.is_ascii_digit();
    let end = chars
        .find(|(_, ch)| ch.is_ascii_digit() != is_digit)
        .map(|(index, _)| start + index)
        .unwrap_or(input.len());
    Some((end, is_digit))
}

/// 数字部分を数値として比較する自然順ソートを行う。
/// たとえば `pad 2` は `pad 11` より前になる。
fn compare_natural_str(left: &str, right: &str) -> Ordering {
    let mut left_index = 0;
    let mut right_index = 0;

    while let (Some((left_end, left_is_digit)), Some((right_end, right_is_digit))) =
        (next_chunk(left, left_index), next_chunk(right, right_index))
    {
        let left_chunk = &left[left_index..left_end];
        let right_chunk = &right[right_index..right_end];

        let ordering = if left_is_digit && right_is_digit {
            let left_trimmed = left_chunk.trim_start_matches('0');
            let right_trimmed = right_chunk.trim_start_matches('0');
            let left_number = if left_trimmed.is_empty() {
                "0"
            } else {
                left_trimmed
            };
            let right_number = if right_trimmed.is_empty() {
                "0"
            } else {
                right_trimmed
            };

            // 数値の桁数 → 数値文字列 → 元の桁数（先頭ゼロの少なさ）の順で比較し、
            // 文字列順ではなく自然順かつ安定した順序にする。
            left_number
                .len()
                .cmp(&right_number.len())
                .then_with(|| left_number.cmp(right_number))
                .then_with(|| left_chunk.len().cmp(&right_chunk.len()))
        } else {
            left_chunk.cmp(right_chunk)
        };

        if ordering != Ordering::Equal {
            return ordering;
        }

        left_index = left_end;
        right_index = right_end;
    }

    left[left_index..].cmp(&right[right_index..])
}

/// 正規化済み・小文字化済みのパッチ名同士を自然順で比較する。
/// ソート時の余分な `String` 確保を避けたい場合はこちらを使う。
pub fn compare_normalized_patch_names_natural(left: &str, right: &str) -> Ordering {
    compare_natural_str(left, right).then_with(|| left.cmp(right))
}

pub fn compare_patch_names_natural(left: &str, right: &str) -> Ordering {
    let normalized_left = normalize_patch_lookup_key(left);
    let normalized_right = normalize_patch_lookup_key(right);

    compare_normalized_patch_names_natural(&normalized_left, &normalized_right)
        .then_with(|| left.cmp(right))
}

/// 保存された patch 名を、いま列挙されている patch の表示名へ突き合わせる。
///
/// prefix 抜きで保存された名前も拾えるよう、`patches_factory` / `patches_3rdparty` を
/// 補ったキーでも探す。
pub fn resolve_display_patch_name(pairs: &[(String, String)], patch_name: &str) -> Option<String> {
    let key = normalize_patch_lookup_key(patch_name);
    if key.is_empty() {
        return None;
    }

    let mut candidates = vec![key.clone()];
    if !PATCH_DIR_PREFIXES
        .iter()
        .any(|prefix| key == *prefix || key.starts_with(&format!("{prefix}/")))
    {
        candidates.extend(
            PATCH_DIR_PREFIXES
                .iter()
                .map(|prefix| format!("{prefix}/{key}")),
        );
    }

    candidates.into_iter().find_map(|candidate| {
        pairs
            .iter()
            .find(|(_, lower)| lower == &candidate)
            .map(|(display, _)| display.clone())
    })
}

#[cfg(test)]
mod tests;
