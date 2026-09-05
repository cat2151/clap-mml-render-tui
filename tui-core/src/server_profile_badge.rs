//! 「いま掴んでいる play server が通常運転ではない」ことを出す右上のバッジ（画面横断で共有）。
//!
//! ## なぜ要るか（実測）
//!
//! debug ビルドの play server で起動していたせいで、DAW の演奏がぶつ切りになった
//! （BPM116 ＝ 小節 2.069 秒での実測）:
//!
//! | サーバー | 先読み 1 小節ぶんの state load | 境界で載せ直した時間 | preload |
//! |---|---|---|---|
//! | debug | 421〜548ms（小節の 23%） | 414〜433ms | miss 8 / hit 8 |
//! | release | 107〜113ms（小節の 5%） | 0.0ms | miss 1 / hit 3 |
//!
//! **どちらが動いているかは、ログの 1 行にしか出ていなかった。** 画面からは一切
//! 分からず、症状から原因へ辿り着くのにログの発掘が要った。詳細は
//! `docs/adr/0017-play-server-binary-resolution.md`。
//!
//! ## 出す条件
//!
//! [`ResolvedServer::needs_attention`] が true のときだけ。素性（`debug` と `不明`）に加えて、
//! **実体がソースより古いとき**も出す。後者は PATH 解決を潰した代わりに生まれた穴で、
//! 「兄弟 repo を直して debug だけ建て、古い release が動き続ける」は
//! `release` = 通常運転なので素性だけでは何も言えなかった。
//! 素性のほうは `release` と `同梱`（配布物の通常形）を静かにしてある。ここを広げると
//! 実ユーザーの通常運転で警告が出っぱなしになり、出っぱなしの警告は読まれなくなる。
//!
//! ## どこから呼ぶか
//!
//! **DAW とそれ以外の両方**。DAW（`DawApp`）は自前の描画ループを持っていて、
//! app 側の overlay を通らない。ぶつ切りが出たのはまさに DAW 画面なので、
//! 片方だけに出しても意味がない。判定も描画もここ 1 か所に置くこと。

use cmrt_realtime_play::{ResolvedServer, ServerBinary};
use ratatui::{
    layout::Rect,
    style::Modifier,
    text::Span,
    widgets::{Clear, Paragraph},
    Frame,
};

use crate::{status::base_style, theme::MONOKAI_PINK};

/// バッジと画面右端のあいだに空ける桁数。
const RIGHT_MARGIN: u16 = 1;

/// 実体のパスとして見せる末尾のセグメント数。
///
/// `target\debug\clap-mml-realtime-play-server.exe` まで出れば、どれを掴んだかは分かる。
/// フルパスは長すぎて、右上の 1 行に収まらない。
const EXE_TAIL_SEGMENTS: usize = 2;

/// 通常運転ではない play server を掴んでいるときだけ、右上へ 1 行出す。
///
/// 呼ぶのは各画面の描画の**最後**。ただし play server の起動失敗 overlay より前
/// （あちらは「音が鳴らない理由」そのものなので、何よりも前面に出す）。
pub fn draw(f: &mut Frame<'_>, binary: &ServerBinary) {
    let Some(resolved) = binary.resolved() else {
        return;
    };
    if !resolved.needs_attention() {
        return;
    }
    // 入らないときは短い形へ落とす。狭い端末で「警告そのものが消える」のが一番まずい。
    let (text, area) = [badge_text(resolved), short_badge_text(resolved)]
        .into_iter()
        .find_map(|text| badge_area(f.area(), &text).map(|area| (text, area)))
        .unzip();
    let (Some(text), Some(area)) = (text, area) else {
        return;
    };
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(Span::styled(
            text,
            base_style().fg(MONOKAI_PINK).add_modifier(Modifier::BOLD),
        )),
        area,
    );
}

/// バッジ 1 行の文字列。
fn badge_text(resolved: &ResolvedServer) -> String {
    format!(
        "{} [{}]",
        short_badge_text(resolved),
        exe_tail(&resolved.exe)
    )
}

/// 実体のパスを落とした短い形。狭い端末ではこちらだけ出す。
///
/// `ソースより古い` は **profile が静かなときでも出る**。そこが今回足した判定で、
/// 「兄弟 repo を直して debug だけ建て、古い release が動き続ける」は
/// `release` = 通常運転なので profile だけでは何も言えなかった。
fn short_badge_text(resolved: &ResolvedServer) -> String {
    let stale = if resolved.stale.is_some() {
        "・ソースより古い"
    } else {
        ""
    };
    format!("⚠ play server: {}{stale}", resolved.profile.label())
}

/// 実体のパスの末尾だけ。区切りは `\` と `/` のどちらの綴りでも同じに扱う。
fn exe_tail(exe: &str) -> String {
    let segments: Vec<&str> = exe.split(['/', '\\']).filter(|s| !s.is_empty()).collect();
    let tail = segments
        .len()
        .saturating_sub(EXE_TAIL_SEGMENTS)
        .min(segments.len());
    segments[tail..].join("/")
}

/// 右上に置く 1 行ぶんの矩形。入らないなら出さない（画面を壊さない）。
fn badge_area(area: Rect, text: &str) -> Option<Rect> {
    if area.height == 0 {
        return None;
    }
    let width = u16::try_from(Span::raw(text).width()).unwrap_or(u16::MAX);
    let available = area.width.saturating_sub(RIGHT_MARGIN);
    if width == 0 || width > available {
        return None;
    }
    Some(Rect {
        x: area.x + available - width,
        y: area.y,
        width,
        height: 1,
    })
}

#[cfg(test)]
mod tests;
