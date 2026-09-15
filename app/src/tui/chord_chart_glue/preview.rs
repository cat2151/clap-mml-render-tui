//! Chord Chart の要求を Chord/Bass の one-shot layer 計画へ変換する。

use cmrt_mml_overlay::{
    line_play::{
        auto_voiced_chord_chart_line_events, chord_chart_line_events, LinePerformance, LineProgram,
        LineStatus,
    },
    LineLayer,
};

use cmrt_chord_chart::PreviewRequest;

use super::{bass_patch::BassPatchResolution, voicing};

const CHORD_INSTANCE: u8 = 0;
const BASS_INSTANCE: u8 = 1;

/// 1 回ぶんの preview を、実際に送る形まで組み立てたもの。
pub(in crate::tui) struct ChordChartPreview {
    /// ログへ残す従来の 1 行。無音要求なら空。
    pub line: String,
    /// Chord layer の解釈結果。Chord の失敗は preview 全体を無音にする。
    pub status: LineStatus,
    /// 従来の Chord-only 計画。Bass OFF の回帰比較にも使う。
    pub program: LineProgram,
    /// Chord layer の canonical patch。
    pub patch: Option<String>,
    /// 同じ command / timeline で sender へ渡す layer 群。
    pub layers: Vec<LineLayer>,
    /// Bass を要求したが layer を作れなかった理由。Chord layer は残す。
    pub bass_reason: Option<String>,
    /// Bass layer が使う patch。未解決または OFF なら `None`。
    pub bass_patch: Option<String>,
    /// 利用者が要求した Bass ON/OFF。失敗しても OFF へ書き換えない。
    pub bass_enabled: bool,
    /// 行全体ではなく chord 1 つに絞ったなら `(0 始まりの番号, 行の chord 総数)`。
    pub chord: Option<(usize, usize)>,
}

pub(super) fn build(
    prefix: &str,
    chord_patch: Option<String>,
    bass_enabled: bool,
    bass_patch_resolution: BassPatchResolution,
    request: &PreviewRequest,
) -> ChordChartPreview {
    if request.is_silent() {
        return ChordChartPreview {
            line: String::new(),
            status: LineStatus::Idle,
            program: LineProgram::silent(),
            patch: chord_patch,
            layers: Vec::new(),
            bass_reason: None,
            bass_patch: None,
            bass_enabled,
            chord: None,
        };
    }

    let chord = request
        .chord_index
        .and_then(|index| chord_at(&request.degrees, index));
    let degrees = chord
        .as_ref()
        .map_or(request.degrees.as_str(), |(degrees, _)| degrees.as_str());
    let line = preview_line(prefix, degrees);
    let selected_chord = chord.as_ref().and(request.chord_index);
    let key = key_token(prefix);
    let voicings = voicing::selected_voicings(key, &request.voicing_context);
    let (status, chord_performance) = match voicings.as_deref() {
        Some(voicings) => {
            auto_voiced_chord_chart_line_events(&request.degrees, key, voicings, selected_chord)
        }
        // Chord Chart の parse failure は MML として読み直さず、必ず無音にする。
        None => chord_chart_line_events(degrees, key),
    };
    let program = LineProgram::once(chord_performance.clone());
    let mut layers = Vec::with_capacity(if bass_enabled { 2 } else { 1 });
    if !chord_performance.is_silent() {
        layers.push(LineLayer {
            instance_id: CHORD_INSTANCE,
            patch: chord_patch.clone(),
            performance: chord_performance,
        });
    }

    let (bass_patch, bass_reason) = if bass_enabled && !program.is_silent() {
        build_bass_layer(
            &mut layers,
            key,
            request,
            selected_chord,
            voicings.as_deref(),
            bass_patch_resolution,
        )
    } else {
        (None, None)
    };

    ChordChartPreview {
        line,
        status,
        program,
        patch: chord_patch,
        layers,
        bass_reason,
        bass_patch,
        bass_enabled,
        chord: chord.map(|(_, total)| (request.chord_index.unwrap_or(0), total)),
    }
}

fn build_bass_layer(
    layers: &mut Vec<LineLayer>,
    key: Option<&str>,
    request: &PreviewRequest,
    selected_chord: Option<usize>,
    voicings: Option<&[cmrt_chord::ChordVoicing]>,
    resolution: BassPatchResolution,
) -> (Option<String>, Option<String>) {
    let Some(voicings) = voicings else {
        return (None, Some("Bass 用 voicing を決定できません".to_string()));
    };
    let patch = match resolution {
        BassPatchResolution::Ready(patch) => patch,
        BassPatchResolution::Loading => {
            return (
                None,
                Some("Bass patch catalog を読み込み中です".to_string()),
            )
        }
        BassPatchResolution::CatalogError(error) => {
            return (None, Some(format!("Bass patch catalog: {error}")))
        }
        BassPatchResolution::NoCandidate => {
            return (None, Some("Bass patch が見つかりません".to_string()))
        }
    };
    let bass = cmrt_chord::timed_chord_progression_performance(key, &request.degrees).and_then(
        |performance| cmrt_chord::bass_timed_progression(&performance, voicings, selected_chord),
    );
    let bass = match bass {
        Ok(performance) if !performance.events.is_empty() => LinePerformance {
            events: performance.events,
            loop_seconds: performance.duration_seconds,
        },
        Ok(_) => {
            return (
                Some(patch),
                Some("voicing に Bass note がありません".to_string()),
            )
        }
        Err(error) => return (Some(patch), Some(format!("Bass performance: {error}"))),
    };
    layers.push(LineLayer {
        instance_id: BASS_INSTANCE,
        patch: Some(patch.clone()),
        performance: bass,
    });
    (Some(patch), None)
}

/// preview 要求 1 つを、ログ 1 行にする。
pub(in crate::tui) fn preview_request_log_line(
    request: &PreviewRequest,
    bass_enabled: bool,
) -> String {
    format!(
        "chord-chart: event=preview-request name=\"{}\" degrees=\"{}\" chord={} bass={}",
        request.name,
        request.degrees,
        request
            .chord_index
            .map_or_else(|| "all".to_string(), |index| index.to_string()),
        if bass_enabled { "on" } else { "off" },
    )
}

/// 実際に何を送る計画にしたかを Chord/Bass ごとに残す。
pub(in crate::tui) fn preview_play_log_line(preview: &ChordChartPreview) -> String {
    let result = match &preview.status {
        LineStatus::Idle => "result=silent".to_string(),
        LineStatus::Played {
            from_chord,
            note_count,
        } => format!("result=played from_chord={from_chord} notes={note_count}"),
        LineStatus::Error(error) => format!("result=error detail=\"{error}\""),
    };
    let chord = preview.chord.map_or_else(
        || "all".to_string(),
        |(index, total)| format!("{index}/{total}"),
    );
    let bass = if !preview.bass_enabled {
        "off"
    } else if preview.bass_reason.is_some() {
        "missing"
    } else {
        "on"
    };
    // `program` は Bass 追加前からの Chord-only 計画そのもの。診断の Chord 数も
    // そこから取り、layer を増やしても従来出力が変わらないことを固定する。
    let chord_notes = note_count(&preview.program.performance);
    let bass_notes = preview
        .layers
        .iter()
        .find(|layer| layer.instance_id == BASS_INSTANCE)
        .map_or_else(Vec::new, |layer| note_numbers(&layer.performance));
    let bass_range = note_number_range(&bass_notes);
    let bass_reason = preview
        .bass_reason
        .as_deref()
        .map_or(String::new(), |reason| format!(" bass_reason=\"{reason}\""));
    format!(
        "chord-chart: event=preview-play line=\"{}\" chord={chord} {result} \
         bass={bass} chord_patch=\"{}\" chord_notes={chord_notes} bass_patch=\"{}\" \
         bass_notes={} bass_note_numbers={bass_notes:?} bass_note_range={bass_range}{bass_reason}",
        preview.line,
        patch_name(preview.patch.as_deref(), "default"),
        patch_name(preview.bass_patch.as_deref(), "none"),
        bass_notes.len(),
    )
}

/// app の command id と sender へ渡した layer 数を結び付ける。
pub(in crate::tui) fn preview_command_log_line(
    preview: &ChordChartPreview,
    command_id: Option<u64>,
) -> String {
    match command_id {
        Some(command_id) => format!(
            "chord-chart: event=preview-command command_id={command_id} status=queued layers={}",
            preview.layers.len()
        ),
        None => format!(
            "chord-chart: event=preview-command command_id=none status=sender-unavailable layers={}",
            preview.layers.len()
        ),
    }
}

/// sender の実演奏区間が、Chord Chart が最後に送った command と一致して鳴っているか。
pub(in crate::tui) fn preview_command_sounding(
    expected_command_id: Option<u64>,
    playback: Option<(u64, bool)>,
) -> bool {
    matches!(
        (expected_command_id, playback),
        (Some(expected), Some((actual, true))) if expected == actual
    )
}

fn note_count(performance: &LinePerformance) -> usize {
    performance
        .events
        .iter()
        .filter(|event| event.message[0] & 0xf0 == 0x90 && event.message[2] > 0)
        .count()
}

fn note_numbers(performance: &LinePerformance) -> Vec<u8> {
    performance
        .events
        .iter()
        .filter(|event| event.message[0] & 0xf0 == 0x90 && event.message[2] > 0)
        .map(|event| event.message[1])
        .collect()
}

fn note_number_range(notes: &[u8]) -> String {
    let Some(min) = notes.iter().min() else {
        return "none".to_string();
    };
    let max = notes
        .iter()
        .max()
        .expect("a minimum means a maximum exists");
    format!("{min}..={max}(span={})", max - min)
}

fn patch_name<'a>(patch: Option<&'a str>, fallback: &'a str) -> &'a str {
    patch.unwrap_or(fallback)
}

fn chord_at(degrees: &str, index: usize) -> Option<(String, usize)> {
    let ranges = cmrt_chord::chord_source_ranges(degrees);
    let range = ranges.get(index)?.clone();
    Some((degrees.get(range)?.to_string(), ranges.len()))
}

pub(in crate::tui) fn preview_line(prefix: &str, degrees: &str) -> String {
    match key_token(prefix) {
        Some(key) => format!("{key} {degrees}"),
        None => degrees.to_string(),
    }
}

pub(in crate::tui) fn key_token(prefix: &str) -> Option<&str> {
    prefix
        .split_whitespace()
        .find(|token| token.to_ascii_lowercase().starts_with("key"))
}
