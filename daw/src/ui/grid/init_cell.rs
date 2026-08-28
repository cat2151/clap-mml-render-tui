//! init 列（meas 0）のセル表示文字列を組み立てる。
//!
//! init セルの実体は `{"Surge XT patch": "<表示パス>"}` という JSON 文字列で、
//! 先頭数文字を出すだけでは全 track が `{"Su` に見えてしまい、どの track が
//! どの音色なのか読み取れなかった。ここで `role:音色名` へ組み直す。
//!
//! 表示だけの責務。セルの値そのものは書き換えない。

use cmrt_patches::PatchRole;
use cmrt_tui_core::patch_load::PatchCatalogSnapshot;

use super::super::super::DawApp;

#[cfg(test)]
mod tests;

/// 表示パスの末尾要素から落とす拡張子。
///
/// **末尾のドット以降を無条件に落としてはいけない。** Dexed の音色名には
/// `05 T.BL-EXPA` や `14 P.ICE 25.1` のようにドットを含むものが実在し、
/// 無条件に落とすと名前が欠ける。既知の拡張子だけを対象にする。
const PATCH_FILE_EXTENSIONS: [&str; 5] = ["fxp", "sfz", "vvp", "syx", "floe-preset"];

/// role 語の綴りを決める唯一の場所。
fn role_prefix(role: PatchRole) -> &'static str {
    role.key()
}

/// 表示パスの末尾要素から、既知の拡張子だけを落とした音色名。
///
/// - `patches_3rdparty/Dan Maurer/Winds/Reed To Pipe Morph.fxp` → `Reed To Pipe Morph`
/// - `SynprezFM/SynprezFM_22.syx/05 SampleSqr2` → `05 SampleSqr2`
pub(super) fn patch_stem(display: &str) -> &str {
    let last = display.rsplit(['/', '\\']).next().unwrap_or(display).trim();
    match last.rsplit_once('.') {
        Some((stem, extension))
            if !stem.is_empty()
                && PATCH_FILE_EXTENSIONS
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(extension)) =>
        {
            stem
        }
        _ => last,
    }
}

/// track 0（Tempo 行）の init セル表示。`{"beat": "4/4"}t120` → `4/4 t120`。
///
/// 拍子が読めないときだけ `None`（＝呼び出し側は生の MML を出す）。
/// テンポは省略可（`t` が無ければ拍子だけを出す）。
fn tempo_label(mml: &str) -> Option<String> {
    let (value, phrase) = DawApp::extract_patch_json_and_phrase(mml)?;
    let beat = value.get("beat")?.as_str()?.trim();
    if beat.is_empty() {
        return None;
    }
    match crate::timing::parse_tempo_bpm(&phrase) {
        Some(bpm) => Some(format!("{beat} t{bpm}")),
        None => Some(beat.to_string()),
    }
}

/// 保存された patch 名を、snapshot の表示名へ突き合わせてから role を引く。
///
/// 短縮名で保存されていると `role_of` が素通しでは当たらないので、
/// 当たらなかったときだけ突き合わせる（当たる場合の線形走査を避ける）。
fn role_of_patch(snapshot: &PatchCatalogSnapshot, patch_name: &str) -> Option<PatchRole> {
    snapshot.patch_roles().role_of(patch_name).or_else(|| {
        let resolved = cmrt_patches::resolve_display_patch_name(snapshot.pairs(), patch_name)?;
        snapshot.patch_roles().role_of(&resolved)
    })
}

/// track 1 以降の init セル表示。`{"Surge XT patch": "..."}` → `lead:Reed To Pipe Morph`。
///
/// catalog がまだ `Loading` / `Err`、あるいは role を引けない音色なら音色名だけを出す。
/// JSON が入っていない（生 MML の）セルは `None`。
fn patch_label(mml: &str, snapshot: Option<&PatchCatalogSnapshot>) -> Option<String> {
    let (patch_name, _phrase) = DawApp::extract_patch_phrase(mml)?;
    let stem = patch_stem(&patch_name);
    let role = snapshot.and_then(|snapshot| role_of_patch(snapshot, &patch_name));
    Some(match role {
        Some(role) => format!("{}:{stem}", role_prefix(role)),
        None => stem.to_string(),
    })
}

/// init セルに出す文字列。`None` なら呼び出し側が生の MML をそのまま詰める。
///
/// 桁詰め・切り詰めは行わない（列幅は `super::cell_width` の責務）。
pub(super) fn init_cell_text(
    track: usize,
    mml: &str,
    snapshot: Option<&PatchCatalogSnapshot>,
) -> Option<String> {
    if track < crate::FIRST_PLAYABLE_TRACK {
        tempo_label(mml)
    } else {
        patch_label(mml, snapshot)
    }
}
