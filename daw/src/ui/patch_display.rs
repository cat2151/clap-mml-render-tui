//! init セル（meas 0）に書かれた音色を、画面に出す `role` と音色名へ読み解く。
//!
//! init 列（`ui/grid/init_cell.rs`）と mixer overlay（`ui/mixer.rs`）の両方が
//! 同じ読み解きを使う。**綴りをずらさないこと**が目的で、grid で `lead:Reed To Pipe Morph`
//! と出ている track が mixer では別の名前に見える、という事態を防ぐ。
//!
//! 表示だけの責務。セルの値そのものは書き換えない。

use cmrt_patches::PatchRole;
use cmrt_tui_core::patch_load::PatchCatalogSnapshot;

use super::super::DawApp;

#[cfg(test)]
mod tests;

/// 生成対象 track の印。
///
/// 色（紫）だけを頼りにすると solo 表示やモノクロ環境で消えるので、文字でも残す。
/// MML に現れない記号を選ぶこと（`>` `<` `#` `+` は MML の一部で、セルの中身と紛れる）。
pub(super) const GENERATED_MARK: &str = "*";

/// 指定が空のときに出す語。キーだけ書かれている状態も見えるようにする。
pub(super) const GENERATED_WITHOUT_DIRECTIVE: &str = "chord";

/// 音色や role が読み取れないときに出す語。空欄にすると
/// 「まだ読み込み中」なのか「列がずれている」のか見分けが付かない。
pub(super) const MISSING: &str = "---";

/// 表示パスの末尾要素から落とす拡張子。
///
/// **末尾のドット以降を無条件に落としてはいけない。** Dexed の音色名には
/// `05 T.BL-EXPA` や `14 P.ICE 25.1` のようにドットを含むものが実在し、
/// 無条件に落とすと名前が欠ける。既知の拡張子だけを対象にする。
const PATCH_FILE_EXTENSIONS: [&str; 5] = ["fxp", "sfz", "vvp", "syx", "floe-preset"];

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

/// 保存された patch 名を、snapshot の表示名へ突き合わせてから role を引く。
///
/// 短縮名で保存されていると `role_of` が素通しでは当たらないので、
/// 当たらなかったときだけ突き合わせる（当たる場合の線形走査を避ける）。
pub(super) fn role_of_patch(
    snapshot: &PatchCatalogSnapshot,
    patch_name: &str,
) -> Option<PatchRole> {
    snapshot.patch_roles().role_of(patch_name).or_else(|| {
        let resolved = cmrt_patches::resolve_display_patch_name(snapshot.pairs(), patch_name)?;
        snapshot.patch_roles().role_of(&resolved)
    })
}

/// init セルから読み解いた、その track の音色の見え方。
pub(super) struct TrackPatchDisplay {
    /// 音色の用途。catalog が `Loading` / `Err` のときや、分類できない音色では `None`。
    role: Option<PatchRole>,
    /// 音色名（拡張子とフォルダを落としたファイル名）。音色が書かれていなければ `None`。
    stem: Option<String>,
    /// chord2mml への指定（`close` / `key:G octave down` など）。
    /// `Some` なら chord 行から MML を生成する track。指定が空文字のこともある。
    directive: Option<String>,
}

impl TrackPatchDisplay {
    /// chord 行から生成する track なら、chord2mml への指定。
    pub(super) fn generated_directive(&self) -> Option<&str> {
        self.directive.as_deref()
    }

    /// mixer overlay の role 行に出す語。
    ///
    /// 生成 track は `*` を付ける（init 列と同じ規則）。role が引けない音色は、
    /// 生成 track なら chord2mml への指定、そうでなければ `---`。
    pub(super) fn role_label(&self) -> String {
        let body = match (self.role, self.generated_directive()) {
            (Some(role), _) => role.key().to_string(),
            (None, Some(directive)) => {
                let directive = directive.trim();
                if directive.is_empty() {
                    GENERATED_WITHOUT_DIRECTIVE.to_string()
                } else {
                    directive.to_string()
                }
            }
            (None, None) => MISSING.to_string(),
        };
        if self.directive.is_some() {
            format!("{GENERATED_MARK}{body}")
        } else {
            body
        }
    }

    /// mixer overlay の音色名行に出す語。
    pub(super) fn patch_label(&self) -> String {
        self.stem.clone().unwrap_or_else(|| MISSING.to_string())
    }

    /// init 列のセルに出す `role:音色名`。音色が書かれていないセルは `None`。
    ///
    /// role が引けないときは音色名だけを出す。
    pub(super) fn init_cell_label(&self) -> Option<String> {
        let stem = self.stem.as_deref()?;
        Some(match self.role {
            Some(role) => format!("{}:{stem}", role.key()),
            None => stem.to_string(),
        })
    }
}

/// 演奏 track の init セルを読み解く。
///
/// `snapshot` が `None`（catalog が `Loading` / `Err`）でも音色名までは読める。
/// role だけが引けなくなる。
pub(super) fn track_patch_display(
    init_cell: &str,
    snapshot: Option<&PatchCatalogSnapshot>,
) -> TrackPatchDisplay {
    let patch_name = DawApp::extract_patch_phrase(init_cell)
        .map(|(patch_name, _phrase)| patch_name)
        .filter(|patch_name| !patch_name.trim().is_empty());
    let role = patch_name
        .as_deref()
        .zip(snapshot)
        .and_then(|(patch_name, snapshot)| role_of_patch(snapshot, patch_name));
    TrackPatchDisplay {
        role,
        stem: patch_name.map(|patch_name| patch_stem(&patch_name).to_string()),
        directive: crate::mml::init_cell_chord_generation_label(init_cell),
    }
}
