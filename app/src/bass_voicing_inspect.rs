//! Chord Chart と同じ auto voicing を、音を鳴らさず note number で検査する。

use anyhow::{bail, Result};

/// 1 回の検査入力。各 `progressions` は Chord Chart の section 1 つに相当し、
/// 全 section を連結してから一度だけ auto voice する。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BassVoicingInspectRequest {
    pub key: String,
    pub progressions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BassStep {
    set_index: usize,
    chord_index: usize,
    symbol: String,
    source_root: u8,
    bass: Option<u8>,
}

/// CLI へ出す診断本文を作る。標準出力と分離し、長い入力もテストでそのまま固定できる。
pub fn report(request: &BassVoicingInspectRequest) -> Result<String> {
    if request.progressions.is_empty() {
        bail!("検査するコード進行を1つ以上指定してください");
    }

    let parsed = request
        .progressions
        .iter()
        .map(|progression| {
            cmrt_chord::parse_chord_progression(&format!("{} {progression}", request.key))
                .map_err(anyhow::Error::msg)
        })
        .collect::<Result<Vec<_>>>()?;
    let all_chords = parsed
        .iter()
        .flat_map(|progression| progression.chords().iter().cloned())
        .collect::<Vec<_>>();
    let key_pitch_class = parsed
        .first()
        .expect("at least one progression was requested")
        .key_pitch_class();
    let voicings = cmrt_chord::auto_voice_with_key(&all_chords, key_pitch_class, None);
    if voicings.len() != all_chords.len() {
        bail!(
            "コード数と voicing 数が一致しません（chords={} voicings={}）",
            all_chords.len(),
            voicings.len()
        );
    }

    let mut steps = Vec::with_capacity(all_chords.len());
    let mut offset = 0;
    for (set_index, progression) in parsed.iter().enumerate() {
        for (chord_index, (symbol, source_notes)) in progression
            .chord_texts()
            .iter()
            .zip(progression.chords())
            .enumerate()
        {
            let Some(source_root) = source_notes.iter().min().copied() else {
                bail!("空のコードを検査できません（set={set_index} chord={chord_index}）");
            };
            steps.push(BassStep {
                set_index,
                chord_index,
                symbol: symbol.clone(),
                source_root,
                bass: voicings[offset + chord_index].bass,
            });
        }
        offset += progression.chords().len();
    }

    Ok(format_report(request, &steps))
}

fn format_report(request: &BassVoicingInspectRequest, steps: &[BassStep]) -> String {
    let basses = steps
        .iter()
        .filter_map(|step| step.bass)
        .collect::<Vec<_>>();
    let mut lines = vec![format!(
        "bass-auto-voicing key={:?} sets={} chords={}",
        request.key,
        request.progressions.len(),
        steps.len()
    )];
    let mut previous_bass: Option<u8> = None;
    for (set_index, progression) in request.progressions.iter().enumerate() {
        let set_steps = steps
            .iter()
            .filter(|step| step.set_index == set_index)
            .collect::<Vec<_>>();
        let set_basses = set_steps
            .iter()
            .filter_map(|step| step.bass)
            .collect::<Vec<_>>();
        lines.push(format!(
            "set-summary set={set_index} degrees={progression:?} \
             bass_note_numbers={set_basses:?} bass_note_range={} max_jump={}",
            note_range(&set_basses),
            display_max_jump(&set_basses),
        ));
        for step in set_steps {
            let jump = match (previous_bass, step.bass) {
                (Some(previous), Some(bass)) => previous.abs_diff(bass).to_string(),
                _ => "none".to_string(),
            };
            lines.push(format!(
                "set={} chord={} symbol={:?} source_root={} root_pc={} bass={} jump={jump}",
                step.set_index,
                step.chord_index,
                step.symbol,
                step.source_root,
                step.source_root % 12,
                optional_note(step.bass),
            ));
            previous_bass = step.bass;
        }
    }
    lines.push(format!(
        "summary bass_note_numbers={basses:?} bass_note_range={} max_jump={}",
        note_range(&basses),
        display_max_jump(&basses),
    ));
    lines.join("\n") + "\n"
}

fn optional_note(note: Option<u8>) -> String {
    note.map(|note| note.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn note_range(notes: &[u8]) -> String {
    let Some(min) = notes.iter().min() else {
        return "none".to_string();
    };
    let max = notes
        .iter()
        .max()
        .expect("a minimum means a maximum exists");
    format!("{min}..={max}(span={})", max - min)
}

fn max_jump(notes: &[u8]) -> Option<u8> {
    notes.windows(2).map(|pair| pair[0].abs_diff(pair[1])).max()
}

fn display_max_jump(notes: &[u8]) -> String {
    max_jump(notes)
        .map(|jump| jump.to_string())
        .unwrap_or_else(|| "none".to_string())
}

#[cfg(test)]
mod tests;
