//! 開いた直後の状態を組み立てる。

use super::*;

use cmrt_patches::PatchRole;

use crate::patch_catalog::sort_for_selector;

/// [`PatchSelect::open`] へ host が渡すもの。
#[derive(Default)]
pub struct PatchSelectRequest {
    pub patches: Vec<PatchCatalogEntry>,
    /// 今の音色。初期カーソルと初期 Preset、取り消しで戻す先に使う。
    pub current: Option<String>,
    /// `(Grid Sequencer 上の役割 group, 正規表現)` のユーザー追加プリセット。
    pub user_presets: Vec<(String, String)>,
    /// 空なら `patches` から作る。
    pub role_index: PatchRoleIndex,
    /// 開いた直後に選ぶ Role。`None` は今の音色の Role（分類できなければ `ALL`）。
    pub initial_role: Option<PatchRole>,
    /// 設定不足でカタログから外れたプラグインの案内。
    pub catalog_notes: Vec<String>,
    pub load_measurements: BTreeMap<String, PatchLoadMeasurement>,
    /// 音色 favorite。登録が新しい順。
    pub favorites: Vec<String>,
    /// Regex 欄の初期値。空なら絞り込まない。
    pub initial_query: String,
    /// auto reverb を扱う host だけが渡す。`None` なら表示行も `e`/`E` も出さない。
    pub auto_reverb: Option<AutoReverbHost>,
}

impl PatchSelect<'_> {
    /// 音色が 1 つも無ければ開かない（`None` を返す）。
    pub fn open(request: PatchSelectRequest) -> Option<Self> {
        let PatchSelectRequest {
            patches: mut all,
            current,
            user_presets,
            mut role_index,
            initial_role,
            catalog_notes,
            load_measurements,
            favorites,
            initial_query,
            auto_reverb,
        } = request;
        if all.is_empty() {
            return None;
        }
        sort_for_selector(&mut all);
        let user_presets = prepare_user_presets(user_presets);
        if role_index.is_empty() {
            role_index = build_role_index(&all, &user_presets);
        }
        let prepared_presets = prepare_presets(&all, &user_presets, &role_index, &favorites);
        let current = current.as_deref();
        // host が Role を指定しなければ、今の音色の Role で開く。
        let role = initial_role.or_else(|| current.and_then(|patch| role_index.role_of(patch)));
        let drum = current
            .and_then(|patch| role_index.drum_role_of(patch))
            .filter(|_| role == Some(PatchRole::Drum));
        let current_index =
            current.and_then(|current| all.iter().position(|patch| patch.display() == current));
        let (group_cursor, preset_cursor) =
            prepared_presets.start_cursors_for_patch(role, drum, current_index);
        let filtered = Arc::clone(&prepared_presets.for_role(group_cursor)[preset_cursor].matches);
        let cursor = current_index
            .and_then(|current| filtered.iter().position(|index| *index == current))
            .unwrap_or(0);
        let mut select = Self {
            all,
            filtered,
            cursor,
            query: text_input::new_single_line_textarea(&initial_query),
            committed_query: initial_query,
            filter_editing: false,
            filter_error: None,
            user_presets,
            role_index,
            prepared_presets,
            group_cursor,
            preset_cursor,
            focus: PatchSelectFocus::Patches,
            scroll_offsets: [Cell::new(0), Cell::new(0), Cell::new(0)],
            original: current.map(str::to_string),
            previewed: current.map(str::to_string),
            catalog_notes,
            load_measurements,
            favorites,
            auto_reverb: auto_reverb.map(AutoReverbPanel::new),
        };
        select.update_filter();
        Some(select)
    }
}

/// Preset を事前計算し、各 Role の preset 1 に `★ Favorite` を入れる。
pub(super) fn prepare_presets(
    all: &[PatchCatalogEntry],
    user_presets: &[(String, String)],
    role_index: &PatchRoleIndex,
    favorites: &[String],
) -> PreparedPresets {
    let mut presets = PreparedPresets::build(all, user_presets, role_index)
        .expect("validated preset regular expressions must compile");
    presets.set_favorites(all, favorites);
    presets
}
