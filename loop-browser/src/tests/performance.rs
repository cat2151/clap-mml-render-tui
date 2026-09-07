use super::*;

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[test]
fn analysis_lookup_is_indexed_for_a_realistic_library() {
    const WAV_COUNT: usize = 6_914;
    let browser = browser_with_direct_wavs(WAV_COUNT);
    let wav = browser.wav_analyses.last().unwrap().0.clone();

    assert_eq!(browser.wav_analysis_indices.len(), WAV_COUNT);
    let started_at = std::time::Instant::now();
    for _ in 0..1_000 {
        assert!(browser.analysis_for_wav(&wav).is_some());
    }
    assert!(
        started_at.elapsed() < std::time::Duration::from_millis(250),
        "indexed lookup exceeded budget: {:?}",
        started_at.elapsed()
    );
}

// ---------------------------------------------------------------------------
// `/` 絞り込みの 1 打鍵あたりの再構築コスト（Stage 7）
//
// デバウンスは禁止（AGENTS.md）なので、1 文字打つたびに
// 「条件のコンパイル + ツリーの刈り取り（`retain_matching` の `clone`） + `collect_visible`」
// が丸ごと走る。ここでその実測値を固定する。
// ---------------------------------------------------------------------------

/// 1 打鍵の予算。
///
/// release は `performance.rs` の `SLOW_RENDER`（50ms）と同じ。これを超えると
/// perf ログの `slow=true` が立ち、体感でも引っかかる。
/// debug は同じ処理が実測で 2〜3 倍かかる（2026-09-07 実測: release 最大 10.5ms /
/// debug 最大 27.0ms）ので、その比のぶんだけ緩める。
fn keystroke_budget() -> Duration {
    if cfg!(debug_assertions) {
        Duration::from_millis(150)
    } else {
        Duration::from_millis(50)
    }
}

/// 2026-09-07 に実ライブラリのキャッシュ（`loop_index.json`）を数えた形。
/// 6,914 wav / 156 ディレクトリ / 相対パスの深さ 3〜6 コンポーネント
/// （深さごとの本数 3,649 / 2,397 / 587 / 281）。
fn realistic_relative_paths() -> Vec<String> {
    let level1 = (0..12)
        .map(|a| format!("Collection{a:02}"))
        .collect::<Vec<_>>();
    let level2 = nest(&level1, 4, |index| {
        // 実ライブラリの `drum` が 1,663 件（24%）ヒットするのは、ファイル名ではなく
        // ディレクトリ名にマッチしてサブツリーごと残るため。その形を再現する。
        if index == 0 {
            "Drums".to_string()
        } else {
            format!("Set{index}")
        }
    });
    let level3 = nest(&level2[..16], 4, |index| format!("Layer{index}"));
    let level4 = nest(&level3[..6], 4, |index| format!("Group{index}"));
    let level5 = nest(&level4[..2], 4, |index| format!("Var{index}"));
    assert_eq!(
        level1.len() + level2.len() + level3.len() + level4.len() + level5.len(),
        156,
        "実ライブラリのディレクトリ数と揃えること"
    );

    let mut paths = Vec::new();
    let mut next = 0usize;
    for (dirs, wavs) in [
        (&level2, 3_649),
        (&level3, 2_397),
        (&level4, 587),
        (&level5, 281),
    ] {
        for wav in 0..wavs {
            let dir = &dirs[wav % dirs.len()];
            paths.push(format!("{dir}/{}{next:04}.wav", wav_stem(next)));
            next += 1;
        }
    }
    assert_eq!(paths.len(), 6_914);
    paths
}

fn nest(parents: &[String], count: usize, name: impl Fn(usize) -> String) -> Vec<String> {
    parents
        .iter()
        .flat_map(|parent| {
            (0..count)
                .map(|index| format!("{parent}/{}", name(index)))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn wav_stem(index: usize) -> &'static str {
    const STEMS: [&str; 8] = [
        "Kick", "Snare", "HiHat", "Clap", "Bass", "Lead", "Perc", "Break",
    ];
    STEMS[index % STEMS.len()]
}

fn browser_with_a_realistic_library() -> LoopBrowser {
    LoopBrowser::from_index(
        LoopIndex {
            version: cmrt_loop_browser_domain::library::LOOP_INDEX_VERSION,
            roots: vec![LoopRootIndex {
                path: "/loops".to_string(),
                wav_files: realistic_relative_paths()
                    .into_iter()
                    .map(indexed)
                    .collect(),
            }],
        },
        &cmrt_runtime::default_loop_categories(),
        cmrt_loop_browser_domain::persisted::PersistedDoc::in_memory(LoopBrowserMetadata::default()),
    )
}

/// `perf_log_line` の行き先を奪って、`log_filter_query` が本当に出ているかを読む。
///
/// `set_log_sinks` は `OnceLock` なのでプロセスに 1 回しか刺さらない。
/// 同じテストバイナリの他のテストと共有するため、行は全部ためて呼び出し側が絞る。
fn perf_log_capture() -> &'static Mutex<Vec<String>> {
    static LINES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    let lines = LINES.get_or_init(|| Mutex::new(Vec::new()));
    fn sink(message: &str) {
        if let Ok(mut lines) = perf_log_capture().lock() {
            lines.push(message.to_string());
        }
    }
    crate::set_log_sinks(|_| {}, sink);
    lines
}

fn captured_lines_for(query: &str) -> Vec<String> {
    perf_log_capture()
        .lock()
        .unwrap()
        .iter()
        .filter(|line| line.contains(&format!("query=\"{query}\"")))
        .cloned()
        .collect()
}

/// 1 打鍵ずつ確定させて、各打鍵の所要時間を返す。
fn type_query(browser: &mut LoopBrowser, query: &str) -> Vec<(String, Duration)> {
    let mut typed = String::new();
    let mut timings = Vec::new();
    for character in query.chars() {
        typed.push(character);
        let started_at = Instant::now();
        browser.set_filter_query(&typed);
        timings.push((typed.clone(), started_at.elapsed()));
    }
    timings
}

fn report(label: &str, browser: &LoopBrowser, timings: &[(String, Duration)]) {
    for (query, elapsed) in timings {
        println!(
            "{label}: query={query:?} {:.3}ms",
            elapsed.as_secs_f64() * 1_000.0
        );
    }
    println!(
        "{label}: visible={} hits={}",
        browser.visible.len(),
        browser.visible.iter().filter(|node| node.is_wav).count()
    );
}

#[test]
fn typing_a_filter_query_stays_under_the_keystroke_budget() {
    let _ = perf_log_capture();
    let mut browser = browser_with_a_realistic_library();
    assert_eq!(browser.wav_analyses.len(), 6_914);

    // 打鍵の途中ほど広くヒットするので、`k` の 1 文字目が最悪ケースになる。
    // 実ライブラリでも `k` は 3,260 件（47%）に当たる。
    let typing = type_query(&mut browser, "kick");
    report("filter/kick", &browser, &typing);
    // ディレクトリ名にマッチしてサブツリーごと残る側の最悪ケース。
    browser.set_filter_query("");
    let by_directory = type_query(&mut browser, "drum");
    report("filter/drum", &browser, &by_directory);
    // 条件が空文字列にマッチすると root ごと残る＝全ノードを組み直す真の最悪ケース。
    browser.set_filter_query("");
    let everything = type_query(&mut browser, "kick|");
    report("filter/kick|", &browser, &everything);

    for (query, elapsed) in typing
        .iter()
        .chain(by_directory.iter())
        .chain(everything.iter())
    {
        assert!(
            *elapsed < keystroke_budget(),
            "1 打鍵の再構築が予算超過: query={query:?} {elapsed:?}"
        );
    }
}

#[test]
fn every_keystroke_writes_one_perf_log_line() {
    let _ = perf_log_capture();
    let mut browser = browser_with_a_realistic_library();

    browser.set_filter_query("Snare0017");
    browser.set_filter_query("Snare0017(");

    let filtered = captured_lines_for("Snare0017");
    assert_eq!(filtered.len(), 1, "{filtered:?}");
    assert!(filtered[0].starts_with("loop-browser-perf: event=filter-query"));
    assert!(filtered[0].contains("outcome=filtered"));
    assert!(filtered[0].contains("library_wavs=6914"));
    assert!(filtered[0].contains("rebuild_ms="));

    // 不正な正規表現でも 1 行出る（結果は直前のまま）。
    let invalid = captured_lines_for("Snare0017(");
    assert_eq!(invalid.len(), 1, "{invalid:?}");
    assert!(invalid[0].contains("outcome=invalid-condition"));
}

/// 実ライブラリでの実測。`CMRT_LOOP_INDEX_JSON` に `loop_index.json` のパスを渡したときだけ走る。
///
/// 個人のパスをコードへ書かないための env 渡し。未設定なら黙って skip する。
#[test]
fn typing_a_filter_query_on_the_real_library_when_the_index_is_given() {
    let Some(path) = std::env::var_os("CMRT_LOOP_INDEX_JSON") else {
        println!("CMRT_LOOP_INDEX_JSON が未設定なので skip");
        return;
    };
    let bytes = std::fs::read(&path).expect("loop_index.json を読めない");
    let index: LoopIndex =
        serde_json::from_slice(&bytes).expect("loop_index.json を parse できない");
    let mut browser = LoopBrowser::from_index(
        index,
        &cmrt_runtime::default_loop_categories(),
        cmrt_loop_browser_domain::persisted::PersistedDoc::in_memory(LoopBrowserMetadata::default()),
    );
    println!("real library: wavs={}", browser.wav_analyses.len());

    for query in ["kick", "drum", "kick|"] {
        browser.set_filter_query("");
        let timings = type_query(&mut browser, query);
        report(&format!("real/{query}"), &browser, &timings);
        for (typed, elapsed) in &timings {
            assert!(
                *elapsed < keystroke_budget(),
                "1 打鍵の再構築が予算超過: query={typed:?} {elapsed:?}"
            );
        }
    }
}
