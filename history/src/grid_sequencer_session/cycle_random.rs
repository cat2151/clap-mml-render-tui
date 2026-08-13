//! 1周ごとに何を引き直すかの wire DTO。domain 側の `CycleRandom` に対応する。

use serde::Serialize;
use serde_json::Value;

/// コード進行1周ごとに引き直す対象。既定は全 ON。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct GridCycleRandomState {
    pub patch: bool,
    pub note: bool,
    pub drum: bool,
    pub arp: bool,
    pub chord: bool,
    pub bpm: bool,
    pub swing: bool,
}

impl Default for GridCycleRandomState {
    fn default() -> Self {
        Self {
            patch: true,
            note: true,
            drum: true,
            arp: true,
            chord: true,
            bpm: true,
            swing: true,
        }
    }
}

/// 欠けた field は既定（ON）として読む。項目を足したセッションを古い版が書き戻しても、
/// 足した項目が黙って OFF にならないため。
pub(super) fn deserialize_cycle_random(value: &Value) -> GridCycleRandomState {
    let default = GridCycleRandomState::default();
    let Some(object) = value.as_object() else {
        return default;
    };
    let flag = |name: &str, fallback: bool| {
        object
            .get(name)
            .and_then(Value::as_bool)
            .unwrap_or(fallback)
    };
    GridCycleRandomState {
        patch: flag("patch", default.patch),
        note: flag("note", default.note),
        drum: flag("drum", default.drum),
        arp: flag("arp", default.arp),
        chord: flag("chord", default.chord),
        bpm: flag("bpm", default.bpm),
        swing: flag("swing", default.swing),
    }
}

/// 旧 `pattern_evolution`（AUTO / HOLD）からの移行。
///
/// HOLD は「譜面も音色も据え置き、進行とテンポだけ動かす」だったので、
/// 譜面まわりの4項目を OFF にする。swing は演奏の質感を毎周変えるもので、
/// 「据え置き」の意図と食い違うので同じく OFF 側。
pub(super) fn migrate_pattern_evolution(value: Option<&Value>) -> GridCycleRandomState {
    if value.and_then(Value::as_str) != Some("hold") {
        return GridCycleRandomState::default();
    }
    GridCycleRandomState {
        patch: false,
        note: false,
        drum: false,
        arp: false,
        chord: true,
        bpm: true,
        swing: false,
    }
}
