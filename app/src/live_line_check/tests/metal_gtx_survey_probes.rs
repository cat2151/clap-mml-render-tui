//! [`super::metal_gtx_survey_capture`] が鳴らす条件の表。

use super::metal_gtx_survey_capture::{probe, sus, Line, Probe, A2, C3};

pub(super) fn probes() -> Vec<Probe> {
    let ks =
        |key: u8, note: u8, seconds: f64| Line::default().ks(key).note(0.0, seconds, note, 100);
    let rel = |shape: u8| sus(A2, 0.5).cc(24, shape).cc(25, 127);
    let stop = || sus(A2, 0.3).cc(24, 104).cc(25, 127);
    let mute = || Line::default().ks(20).note(0.0, 1.0, A2, 100);
    let unison = |cc28: u8| Line::default().ks(94).note(0.0, 2.0, 72, 100).cc(28, cc28);
    vec![
        // 未実装の KS。
        probe("ks-e-1-chromatic-run", "sus-a1", ks(4, 33, 1.0), 0.0, 1.0),
        probe(
            "ks-f-1-slide-fx-down",
            "sus-f#3",
            Line::default().ks(5).note(0.0, 1.0, 54, 90),
            0.0,
            1.0,
        ),
        probe("ks-f#-1-slide-fx-up", "sus-f#2", ks(6, 42, 1.0), 0.0, 1.0),
        probe("ks-g-1-slide-fx-wow", "sus-f#2", ks(7, 42, 1.0), 0.0, 1.0),
        probe(
            "ks-a-1-natural-harmonics",
            "sus-e4",
            ks(9, 64, 1.0),
            0.0,
            1.0,
        ),
        probe("ks-b-1-brush-down", "sus-a2", ks(11, A2, 1.0), 0.0, 1.0),
        probe("ks-c0-brush-up", "sus-a2", ks(12, A2, 1.0), 0.0, 1.0),
        probe(
            "ks-c#0-brush-alt-2nd",
            "sus-a2-x2",
            Line::default()
                .ks(13)
                .note(0.0, 0.5, A2, 100)
                .note(0.5, 1.0, A2, 100),
            0.5,
            0.5,
        ),
        probe("ks-d0-mute-fret-down", "sus-a2", ks(14, A2, 1.0), 0.0, 1.0),
        probe("ks-d#0-mute-fret-up", "sus-a2", ks(15, A2, 1.0), 0.0, 1.0),
        probe(
            "ks-e0-mute-fret-alt-2nd",
            "sus-a2-x2",
            Line::default()
                .ks(16)
                .note(0.0, 0.5, A2, 100)
                .note(0.5, 1.0, A2, 100),
            0.5,
            0.5,
        ),
        probe(
            "ks-g0-sus-alt-2nd",
            "sus-a2-x2",
            Line::default()
                .ks(19)
                .note(0.0, 0.5, A2, 100)
                .note(0.5, 1.0, A2, 100),
            0.5,
            0.5,
        ),
        probe(
            "ks-a#0-mute-alt-2nd",
            "mute-a2-x2",
            Line::default()
                .ks(22)
                .note(0.0, 0.5, A2, 100)
                .note(0.5, 1.0, A2, 100),
            0.5,
            0.5,
        ),
        probe("ks-d#1-slide-in", "sus-a2", ks(27, A2, 1.0), 0.0, 1.0),
        probe("ks-e1-slide-out", "sus-a2", ks(28, A2, 1.0), 0.0, 1.0),
        probe(
            "ks-f1-pseudo-legato-2nd",
            "legato-a2-c3",
            Line::default()
                .ks(29)
                .note(0.0, 0.5, A2, 100)
                .note(0.5, 1.0, C3, 100),
            0.5,
            0.5,
        ),
        probe(
            "ks-a#6-unison-bend-auto",
            "sus-c5-long",
            ks(94, 72, 2.0),
            0.0,
            2.0,
        ),
        probe(
            "ks-b6-unison-bend-manual",
            "sus-c5-long",
            ks(95, 72, 2.0),
            0.0,
            2.0,
        ),
        probe(
            "ks-b6-unison-bend-manual-pb",
            "sus-c5-long",
            ks(95, 72, 2.0).bend_at(0.3, 8191),
            0.0,
            2.0,
        ),
        probe(
            "ks-c7-portament-2nd",
            "legato-a2-c3",
            Line::default()
                .ks(96)
                .note(0.0, 0.5, A2, 100)
                .note(0.5, 1.0, C3, 100),
            0.5,
            0.5,
        ),
        probe(
            "ks-c#7-pbr12",
            "sus-a2-bend",
            ks(97, A2, 1.0).bend_at(0.3, 8191),
            0.35,
            0.6,
        ),
        probe(
            "ks-d7-pbr24",
            "sus-a2-bend",
            ks(98, A2, 1.0).bend_at(0.3, 8191),
            0.35,
            0.6,
        ),
        probe("ks-d#7-trill-ht", "sus-a2-long", ks(99, A2, 2.0), 0.0, 2.0),
        probe("ks-e7-trill-wt", "sus-a2-long", ks(100, A2, 2.0), 0.0, 2.0),
        probe(
            "ks-f7-trill-min3",
            "sus-a2-long",
            ks(101, A2, 2.0),
            0.0,
            2.0,
        ),
        probe(
            "ks-f#7-trill-maj3",
            "sus-a2-long",
            ks(102, A2, 2.0),
            0.0,
            2.0,
        ),
        // KS の下の直接鳴らす効果音（KSMap の 0〜3）。
        probe(
            "fx-c-1-hello",
            "silence",
            Line::default().note(0.0, 1.0, 0, 100),
            0.0,
            1.0,
        ),
        probe(
            "fx-c#-1-resonance",
            "silence",
            Line::default().note(0.0, 1.0, 1, 100),
            0.0,
            1.0,
        ),
        probe(
            "fx-d-1-slide-noise",
            "silence",
            Line::default().note(0.0, 1.0, 2, 100),
            0.0,
            1.0,
        ),
        probe(
            "fx-d#-1-hard-stop",
            "silence",
            Line::default().note(0.0, 1.0, 3, 100),
            0.0,
            1.0,
        ),
        // リリース音を止める方法（説明書「C0を短くまたは…弱く打ち込む」）。
        probe(
            "stop-weak-note-sus",
            "stop-rep-none",
            stop().note(0.8, 0.85, A2, 5),
            0.85,
            0.7,
        ),
        probe(
            "stop-weak-note-e-1-stop-key",
            "stop-rep-none",
            stop().ks_at(0.75, 4).note(0.8, 0.85, A2, 5),
            0.85,
            0.7,
        ),
        probe(
            "stop-ks-c0-short",
            "stop-rep-none",
            stop().ks_at(0.8, 12),
            0.85,
            0.7,
        ),
        // CC。
        probe(
            "cc21-0",
            "vib-a2",
            sus(A2, 2.0).cc(20, 100).cc(21, 0),
            0.0,
            2.0,
        ),
        probe(
            "cc21-127",
            "vib-a2",
            sus(A2, 2.0).cc(20, 100).cc(21, 127),
            0.0,
            2.0,
        ),
        probe(
            "cc22-0-mute",
            "mute-a2",
            Line::default().ks(20).note(0.0, 1.0, A2, 100).cc(22, 0),
            0.0,
            1.0,
        ),
        probe(
            "cc22-127-mute",
            "mute-a2",
            Line::default().ks(20).note(0.0, 1.0, A2, 100).cc(22, 127),
            0.0,
            1.0,
        ),
        probe(
            "cc23-127-v100-sus-lt",
            "sus-a2-long",
            sus(A2, 2.0).cc(23, 127),
            0.0,
            2.0,
        ),
        probe(
            "cc23-127-v120-sus-ex",
            "sus-a2-v120",
            Line::default().note(0.0, 1.0, A2, 120).cc(23, 127),
            0.0,
            1.0,
        ),
        probe(
            "cc23-127-mute-ex",
            "mute-a2",
            Line::default().ks(20).note(0.0, 1.0, A2, 100).cc(23, 127),
            0.0,
            1.0,
        ),
        probe("cc24-72-position-change", "rel-none", rel(72), 0.5, 1.0),
        probe("cc24-88-auto-slide-out", "rel-none", rel(88), 0.5, 1.0),
        probe("cc24-104-auto-alternate", "rel-none", rel(104), 0.5, 1.0),
        probe("cc24-120-auto-alternate", "rel-none", rel(120), 0.5, 1.0),
        probe(
            "cc24-72-mute",
            "rel-mute-none",
            Line::default()
                .ks(20)
                .note(0.0, 0.5, A2, 100)
                .cc(24, 72)
                .cc(25, 127),
            0.5,
            1.0,
        ),
        probe(
            "cc24-104-mute",
            "rel-mute-none",
            Line::default()
                .ks(20)
                .note(0.0, 0.5, A2, 100)
                .cc(24, 104)
                .cc(25, 127),
            0.5,
            1.0,
        ),
        probe(
            "cc27-104-slide-in",
            "slide-in-cc27-8",
            Line::default().ks(27).note(0.0, 1.0, A2, 100).cc(27, 104),
            0.0,
            1.0,
        ),
        probe(
            "cc28-127-trill",
            "trill-ht-cc28-0",
            Line::default().ks(99).note(0.0, 2.0, A2, 100).cc(28, 127),
            0.0,
            2.0,
        ),
        probe(
            "cc28-127-unison-auto",
            "unison-auto-cc28-0",
            Line::default().ks(94).note(0.0, 2.0, 72, 100).cc(28, 127),
            0.0,
            2.0,
        ),
        probe("cc29-0", "sus-a2", sus(A2, 1.0).cc(29, 0), 0.0, 1.0),
        probe("cc29-127", "sus-a2", sus(A2, 1.0).cc(29, 127), 0.0, 1.0),
        probe(
            "cc31-127-attack",
            "sus-a2",
            sus(A2, 1.0).cc(31, 127),
            0.0,
            1.0,
        ),
        probe(
            "cc31-127-f5-rep-release",
            "f5-rep-cc31-0",
            sus(77, 0.5).cc(24, 104).cc(31, 127),
            0.5,
            1.0,
        ),
        probe("cc32-127-p5", "sus-a2", sus(A2, 1.0).cc(32, 127), 0.0, 1.0),
        probe(
            "cc46-127-tension",
            "sus-a2",
            sus(A2, 1.0).cc(46, 127),
            0.0,
            1.0,
        ),
        probe("cc48-0-magnet", "sus-a2", sus(A2, 1.0).cc(48, 0), 0.0, 1.0),
        probe(
            "cc48-127-magnet",
            "sus-a2",
            sus(A2, 1.0).cc(48, 127),
            0.0,
            1.0,
        ),
        probe(
            "cc52-127-bend-start",
            "bend-ht-cc52-0",
            Line::default().ks(91).note(0.0, 2.0, A2, 100).cc(52, 127),
            0.0,
            2.0,
        ),
        probe(
            "cc53-127-bend-speed",
            "bend-ht-cc53-0",
            Line::default().ks(91).note(0.0, 2.0, A2, 100).cc(53, 127),
            0.0,
            2.0,
        ),
        probe(
            "cc112-127-trill-speed",
            "trill-ht-cc112-0",
            Line::default().ks(99).note(0.0, 2.0, A2, 100).cc(112, 127),
            0.0,
            2.0,
        ),
        // 2 巡目: ミュートは減衰が速いので頭だけを比べる。
        probe("r2-mute-head-same", "mute-a2", mute(), 0.0, 0.25),
        probe(
            "r2-cc23-127-mute-ex-head",
            "mute-a2",
            mute().cc(23, 127),
            0.0,
            0.25,
        ),
        probe(
            "r2-cc22-127-mute-head",
            "mute-a2",
            mute().cc(22, 127),
            0.0,
            0.25,
        ),
        probe(
            "r2-cc22-0-mute-head",
            "mute-a2",
            mute().cc(22, 0),
            0.0,
            0.25,
        ),
        probe(
            "r2-ks-a#0-mute-alt-2nd-head",
            "mute-a2-x2",
            Line::default()
                .ks(22)
                .note(0.0, 0.5, A2, 100)
                .note(0.5, 1.0, A2, 100),
            0.5,
            0.25,
        ),
        probe(
            "r2-mute-x2-2nd-head-same",
            "mute-a2-x2",
            Line::default()
                .ks(20)
                .note(0.0, 0.5, A2, 100)
                .note(0.5, 1.0, A2, 100),
            0.5,
            0.25,
        ),
        // 2 巡目: 自動ユニゾンは同じ条件でも揺れるので、同じ条件を重ねて揺れの幅を見る。
        probe(
            "r2-unison-cc28-0-same-1",
            "unison-auto-cc28-0",
            unison(0),
            0.0,
            2.0,
        ),
        probe(
            "r2-unison-cc28-0-same-2",
            "unison-auto-cc28-0",
            unison(0),
            0.0,
            2.0,
        ),
        probe(
            "r2-unison-cc28-127-1",
            "unison-auto-cc28-0",
            unison(127),
            0.0,
            2.0,
        ),
        probe(
            "r2-unison-cc28-127-2",
            "unison-auto-cc28-0",
            unison(127),
            0.0,
            2.0,
        ),
        // 2 巡目: 共鳴は小さいので、静かなフレットミュートの上で比べる。
        probe(
            "r2-cc29-127-mute-fret",
            "mute-fret-cc29-0",
            Line::default().ks(14).note(0.0, 1.0, A2, 100).cc(29, 127),
            0.0,
            1.0,
        ),
    ]
}
