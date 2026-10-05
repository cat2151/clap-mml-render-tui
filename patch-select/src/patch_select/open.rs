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
    /// 今の音色の Role が分からないときに開く Role。`None` なら `ALL`。
    pub initial_role: Option<PatchRole>,
    /// Drum tracks / Drum kit に固定し、候補移動・検索・確定・取消だけを使う。
    /// kit は名前の Role にかかわらず `drum_kit=true` の全 patch。
    pub drum_kit_only: bool,
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
            drum_kit_only,
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
        let prepared_presets = prepare_presets(
            &all,
            &user_presets,
            &role_index,
            &favorites,
            &load_measurements,
        );
        let current = current.as_deref();
        // 今の音色の Role で開く。分からなければ host が指定した Role。
        let role = current
            .and_then(|patch| role_index.role_of(patch))
            .or(initial_role);
        let drum = current
            .and_then(|patch| role_index.drum_role_of(patch))
            .filter(|_| role == Some(PatchRole::Drum));
        let current_index =
            current.and_then(|current| all.iter().position(|patch| patch.display() == current));
        let (group_cursor, preset_cursor) = if drum_kit_only {
            let group = FilterGroup::ALL
                .iter()
                .position(|group| group.role() == Some(PatchRole::Drum))?;
            let preset = prepared_presets
                .for_role(group)
                .iter()
                .position(|preset| preset.is_drum_kit)?;
            (group, preset)
        } else {
            prepared_presets.start_cursors_for_patch(role, drum, current_index)
        };
        let filtered = Arc::clone(&prepared_presets.for_role(group_cursor)[preset_cursor].matches);
        let cursor = current_index
            .and_then(|current| filtered.iter().position(|index| *index == current))
            .unwrap_or(0);
        let auto_reverb = auto_reverb.filter(|_| !drum_kit_only).map(|host| {
            let mut panel = AutoReverbPanel::new(host);
            panel.set_user_presets(&user_presets);
            panel
        });
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
            drum_kit_only,
            group_cursor,
            preset_cursor,
            focus: PatchSelectFocus::Patches,
            scroll_offsets: [Cell::new(0), Cell::new(0), Cell::new(0)],
            original: current.map(str::to_string),
            previewed: current.map(str::to_string),
            catalog_notes,
            load_measurements,
            favorites,
            auto_reverb,
            plugin_menu: None,
        };
        select.update_filter();
        Some(select)
    }
}

/// Preset を事前計算し、各 Role の preset 1 に `★ Favorite` を、Drum には `Drum kit` を入れる。
pub(super) fn prepare_presets(
    all: &[PatchCatalogEntry],
    user_presets: &[(String, String)],
    role_index: &PatchRoleIndex,
    favorites: &[String],
    load_measurements: &BTreeMap<String, PatchLoadMeasurement>,
) -> PreparedPresets {
    let mut presets = PreparedPresets::build(all, user_presets, role_index)
        .expect("validated preset regular expressions must compile");
    presets.set_favorites(all, favorites);
    presets.set_drum_kits(all, load_measurements);
    presets
}
