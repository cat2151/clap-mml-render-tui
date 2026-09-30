//! Guitar Articulation 画面が鳴らした演奏を、画面を開かずに作り直して表で出す。
//!
//! 変換は MML とルール表だけで決まる純関数なので、ログには入力（MML・ルール表）だけを残し、
//! イベント列はここで作り直す。ログの行の書き方と読み方を同じ場所に置く。

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

use cmrt_guitar_articulation::{Articulation, RuleTable, Take, TimedMidiEvent};

const PLAY_LOG_PREFIX: &str = "guitar-articulation: event=play ";
/// raw の演奏はルール表を使わないので、作り直す対象は Articulated の演奏だけ。
const CONVERTED_TAKE: &str = "take=converted ";
const MML_KEY: &str = " mml=";
const RULES_KEY: &str = " rules=";

/// 1 回の出力の入力。`last_played` なら MML とルール表をログの最後の演奏から拾う。
/// `compare_previous` なら、最後の演奏と、同じ MML でルール表が違う直前の演奏を並べる。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuitarArticulationEventsRequest {
    pub last_played: bool,
    pub compare_previous: bool,
    /// ルール表の JSON（[`RuleTable::to_json`] の綴り）。省略時はルール無し。
    pub rules: Option<String>,
    pub mml: Option<String>,
}

/// 送った演奏のログ 1 行。音は機械で判定できないので、何をどの音色（`patch`）へ送ったかを残す。
/// 末尾の `mml` と `rules` は JSON で、[`logged_plays`] がここから演奏を作り直す。
pub(crate) fn play_log_line(
    patch: &str,
    take: Take,
    events: &[TimedMidiEvent],
    mml: &str,
    rules: &RuleTable,
) -> String {
    format!(
        "{PLAY_LOG_PREFIX}{}{MML_KEY}{}{RULES_KEY}{}",
        take_summary(patch, take, events),
        serde_json::Value::from(mml),
        rules.to_json(),
    )
}

/// カーソル列の 1 音だけを送った演奏のログ 1 行。作り直しの対象ではないので MML とルール表は書かず、
/// `event=play-note` にして [`logged_plays`] に拾わせない。
pub(crate) fn play_note_log_line(
    patch: &str,
    take: Take,
    column: usize,
    events: &[TimedMidiEvent],
) -> String {
    format!(
        "guitar-articulation: event=play-note column={column} {}",
        take_summary(patch, take, events)
    )
}

/// サンプル MID を送った演奏のログ 1 行。`note` は全体なら `all`、1 音ならまとまりの番号。
/// MML から作り直せないので `event=play-sample-midi` にして [`logged_plays`] に拾わせない。
pub(crate) fn play_sample_midi_log_line(
    file: &str,
    note: Option<usize>,
    events: &[TimedMidiEvent],
    patch: &str,
) -> String {
    let note = note.map_or_else(|| "all".to_string(), |note| note.to_string());
    let seconds = events.last().map_or(0.0, |event| event.seconds);
    format!(
        "guitar-articulation: event=play-sample-midi file={file:?} note={note} events={} seconds={seconds:.3} patch={patch:?}",
        events.len()
    )
}

/// `take=... events=... keyswitch_note_ons=... seconds=... patch=...`
fn take_summary(patch: &str, take: Take, events: &[TimedMidiEvent]) -> String {
    let keyswitches = events
        .iter()
        .filter(|event| {
            event.message[0] & 0xF0 == 0x90
                && event.message[2] != 0
                && Articulation::from_keyswitch(event.message[1]).is_some()
        })
        .count();
    let seconds = events.last().map_or(0.0, |event| event.seconds);
    format!(
        "take={} events={} keyswitch_note_ons={keyswitches} seconds={seconds:.3} patch={patch:?}",
        take.label(),
        events.len(),
    )
}

/// ログから読み戻した Articulated の演奏 1 回。
#[derive(Debug)]
pub(crate) struct LoggedPlay<'a> {
    pub(crate) line: &'a str,
    pub(crate) mml: String,
    pub(crate) rules: RuleTable,
}

/// 新しい順の Articulated の演奏。MML を残す前のログの行は飛ばす。
fn logged_plays(log: &str) -> impl Iterator<Item = LoggedPlay<'_>> {
    log.lines()
        .rev()
        .filter(|line| line.contains(PLAY_LOG_PREFIX) && line.contains(CONVERTED_TAKE))
        .filter_map(|line| {
            parse_play_log_line(line).map(|(mml, rules)| LoggedPlay { line, mml, rules })
        })
}

pub(crate) fn last_played(log: &str) -> Result<LoggedPlay<'_>> {
    logged_plays(log).next().ok_or_else(no_play_error)
}

/// (前, 最後)。前は、最後より前で同じ MML・違うルール表の最も新しい演奏（同じ演奏の鳴らし直しは飛ばす）。
pub(crate) fn last_two_distinct(log: &str) -> Result<(LoggedPlay<'_>, LoggedPlay<'_>)> {
    let mut plays = logged_plays(log);
    let last = plays.next().ok_or_else(no_play_error)?;
    let previous = plays
        .find(|play| play.mml == last.mml && play.rules != last.rules)
        .ok_or_else(|| anyhow!("最後の演奏と同じ MML で、ルールが違う演奏がログにありません"))?;
    Ok((previous, last))
}

fn no_play_error() -> anyhow::Error {
    anyhow!("ログに MML 付きの guitar-articulation の Articulated の演奏がありません（画面で一度鳴らしてください）")
}

fn parse_play_log_line(line: &str) -> Option<(String, RuleTable)> {
    let rest = &line[line.find(MML_KEY)? + MML_KEY.len()..];
    let (mml, rest) = leading_json::<String>(rest)?;
    let rules_json = rest.strip_prefix(RULES_KEY)?;
    let (rules, _) = leading_json::<RuleTable>(rules_json)?;
    Some((mml, rules))
}

/// 文字列の先頭の JSON 値 1 つと、その後ろの残り。
fn leading_json<'a, T: Deserialize<'a>>(text: &'a str) -> Option<(T, &'a str)> {
    let mut stream = serde_json::Deserializer::from_str(text).into_iter::<T>();
    let value = stream.next()?.ok()?;
    Some((value, &text[stream.byte_offset()..]))
}

/// CLI の本文。ログから作ったときは、どのログ行から作ったかを先頭に出す。
pub fn report(request: &GuitarArticulationEventsRequest) -> Result<String> {
    if request.compare_previous {
        let log = read_log()?;
        let (previous, last) = last_two_distinct(&log)?;
        let body = cmrt_guitar_articulation::compare(&last.mml, &previous.rules, &last.rules)
            .map_err(anyhow::Error::msg)?;
        return Ok(format!(
            "before log: {}\nafter log:  {}\n{body}",
            previous.line, last.line
        ));
    }
    if request.last_played {
        let log = read_log()?;
        let play = last_played(&log)?;
        let body =
            cmrt_guitar_articulation::report(&play.mml, &play.rules).map_err(anyhow::Error::msg)?;
        return Ok(format!("log: {}\n{body}", play.line));
    }
    let Some(mml) = &request.mml else {
        bail!("MML か --last-played / --compare-previous を指定してください");
    };
    let rules = match &request.rules {
        Some(json) => RuleTable::from_json(json).map_err(anyhow::Error::msg)?,
        None => RuleTable::default(),
    };
    cmrt_guitar_articulation::report(mml, &rules).map_err(anyhow::Error::msg)
}

fn read_log() -> Result<String> {
    let path = crate::config::log_file_path().context("ログの置き場が分かりません")?;
    let bytes = std::fs::read(&path).with_context(|| format!("{} を読めません", path.display()))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests;
