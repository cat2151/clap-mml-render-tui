use super::*;

use std::time::{SystemTime, UNIX_EPOCH};

/// 出荷プリセットと同じ形の最小 `.vvp`。`m_uPolyMode` はヘッダのすぐ後ろにある。
fn vvp_xml(name: &str, poly_mode: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n\
         <VASTvaporizer2 PatchVersion=\"VASTVaporizerParamsV2.20000\" PatchName=\"{name}\" \
         PatchCategory=\"PD\" PatchTag=\"Factory\" PatchAuthor=\"tester\">\r\n\
         <PARAM id=\"m_uMasterVolume\" text=\"0.5\"/>\r\n\
         <PARAM id=\"m_uPolyMode\" text=\"{poly_mode}\"/>\r\n\
         </VASTvaporizer2>\r\n"
    )
}

struct PresetDir {
    root: PathBuf,
}

impl PresetDir {
    fn new(label: &str) -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!("cmrt_vvp_voicings_{label}_{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn write(&self, name: &str, poly_mode: &str) {
        std::fs::write(self.root.join(name), vvp_xml(name, poly_mode)).unwrap();
    }

    fn plugin(&self) -> CatalogPlugin {
        CatalogPlugin {
            name: "Vaporizer2".to_string(),
            plugin_path: "VASTvaporizer2.clap".to_string(),
            plugin_id: Some(cmrt_runtime::VAPORIZER2_PLUGIN_ID.to_string()),
            base: Some(self.root.to_string_lossy().into_owned()),
            dirs: vec![self.root.to_string_lossy().into_owned()],
            resolved_patches: None,
            source_notices: Vec::new(),
            patch_roles: cmrt_runtime::PatchRoles::default(),
        }
    }
}

impl Drop for PresetDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn pair(display: &str) -> (String, String) {
    (display.to_string(), display.to_lowercase())
}

#[test]
fn the_poly_mode_written_in_the_file_decides_the_voicing() {
    let dir = PresetDir::new("decides");
    dir.write("PD Wide.vvp", "Poly16");
    dir.write("LD Screamer.vvp", "Mono");
    let plugin = dir.plugin();
    let voicings = VvpVoicings::default();

    assert_eq!(
        voicings.voicing(&plugin, "PD Wide.vvp"),
        Some(PatchVoicing::Poly)
    );
    assert_eq!(
        voicings.voicing(&plugin, "LD Screamer.vvp"),
        Some(PatchVoicing::Mono)
    );
}

/// `Poly4` / `Poly32` も poly。判定は「`Mono` かどうか」であって綴りの一覧ではない。
#[test]
fn every_poly_mode_other_than_mono_is_poly() {
    let dir = PresetDir::new("modes");
    let plugin = dir.plugin();
    let voicings = VvpVoicings::default();

    for mode in ["Poly4", "Poly16", "Poly32"] {
        let name = format!("SY {mode}.vvp");
        dir.write(&name, mode);
        assert_eq!(
            voicings.voicing(&plugin, &name),
            Some(PatchVoicing::Poly),
            "{mode}"
        );
    }
}

/// 読めなかった音色は**未判定**（`None`）にする。poly へ倒すと Mono が和音行へ出る。
#[test]
fn an_unreadable_preset_stays_undecided() {
    let dir = PresetDir::new("unreadable");
    let plugin = dir.plugin();
    let voicings = VvpVoicings::default();

    // そもそもファイルが無い。
    assert_eq!(voicings.voicing(&plugin, "PD Missing.vvp"), None);

    // ファイルはあるが Vaporizer2 の XML ではない。
    std::fs::write(dir.root.join("PD Broken.vvp"), b"not xml at all").unwrap();
    assert_eq!(voicings.voicing(&plugin, "PD Broken.vvp"), None);
}

/// 一度読んだ結果は memo に残る。**読めなかったことも残る**（開き直さない）。
#[test]
fn a_result_is_read_once_even_when_the_file_could_not_be_read() {
    let dir = PresetDir::new("memo");
    dir.write("PD Wide.vvp", "Poly16");
    let plugin = dir.plugin();
    let voicings = VvpVoicings::default();

    assert_eq!(
        voicings.voicing(&plugin, "PD Wide.vvp"),
        Some(PatchVoicing::Poly)
    );
    assert_eq!(voicings.voicing(&plugin, "PD Missing.vvp"), None);

    // ファイルを消しても差し替えても、memo にある答えは変わらない。
    std::fs::remove_file(dir.root.join("PD Wide.vvp")).unwrap();
    dir.write("PD Missing.vvp", "Poly16");

    assert_eq!(
        voicings.voicing(&plugin, "PD Wide.vvp"),
        Some(PatchVoicing::Poly)
    );
    assert_eq!(voicings.voicing(&plugin, "PD Missing.vvp"), None);
    assert_eq!(voicings.memo.lock().unwrap().len(), 2);
}

/// memo は `Arc` 共有。バックグラウンドで先読みした結果を画面側が引けること。
#[test]
fn a_clone_shares_the_same_memo() {
    let dir = PresetDir::new("shared");
    dir.write("PD Wide.vvp", "Mono");
    let plugin = dir.plugin();
    let loader = VvpVoicings::default();
    let screen = loader.clone();

    assert_eq!(
        loader.voicing(&plugin, "PD Wide.vvp"),
        Some(PatchVoicing::Mono)
    );
    std::fs::remove_file(dir.root.join("PD Wide.vvp")).unwrap();

    assert_eq!(
        screen.voicing(&plugin, "PD Wide.vvp"),
        Some(PatchVoicing::Mono)
    );
}

/// 先読みは `.vvp` だけを開く。`.fxp` / cartridge を開きに行くと、拡張子の違う
/// ファイルを Vaporizer2 のヘッダとして読もうとして無駄なエラーが並ぶ。
#[test]
fn prefetch_only_reads_the_vvp_entries() {
    let dir = PresetDir::new("prefetch");
    dir.write("PD Wide.vvp", "Poly16");
    dir.write("LD Screamer.vvp", "Mono");
    let plugins = PatchPlugins::from_catalog(vec![dir.plugin()]);
    let voicings = VvpVoicings::default();

    let read = voicings.prefetch(
        &plugins,
        &[
            pair("PD Wide.vvp"),
            pair("LD Screamer.vvp"),
            pair("Keys/Bright.fxp"),
            pair("SynprezFM_01.syx/00 Say Again."),
        ],
    );

    assert_eq!(read, 2);
    let memo = voicings.memo.lock().unwrap();
    assert_eq!(memo.len(), 2);
    assert_eq!(memo.get("PD Wide.vvp"), Some(&Some(PatchVoicing::Poly)));
    assert_eq!(memo.get("LD Screamer.vvp"), Some(&Some(PatchVoicing::Mono)));
}

/// 基点を持たないプラグインの display は絶対パスそのもの。
#[test]
fn a_catalog_without_a_relative_base_reads_the_absolute_display_path() {
    let dir = PresetDir::new("nobase");
    dir.write("PD Wide.vvp", "Poly16");
    let mut plugin = dir.plugin();
    plugin.base = None;
    let display = dir.root.join("PD Wide.vvp").to_string_lossy().into_owned();

    assert_eq!(
        VvpVoicings::default().voicing(&plugin, &display),
        Some(PatchVoicing::Poly)
    );
}

/// 実際にインストールされている `.vvp` を全部読んでみる。
///
/// **「先頭 4096 バイトで足りるか」「XML の読み方が実データと合っているか」は、
/// これでしか分からない。** 読めなかった音色は未判定として静かに和音行から外れるだけで、
/// 画面には（他の役の候補として）出るので気づけない。
///
/// 音色置き場は個人のパスなのでコードに書かず、`CMRT_TEST_VAPORIZER2_PRESETS` で渡す。
/// 未設定なら `#[ignore]`（通常の `cargo test` では走らない）。
#[test]
#[ignore = "実際にインストールされた Vaporizer2 のプリセットが要る"]
fn every_installed_preset_reports_a_voicing() {
    let Some(dir) = std::env::var_os("CMRT_TEST_VAPORIZER2_PRESETS") else {
        panic!("CMRT_TEST_VAPORIZER2_PRESETS に音色置き場を渡すこと");
    };
    let root = PathBuf::from(&dir);
    let mut names: Vec<String> = std::fs::read_dir(&root)
        .expect("音色置き場を読めない")
        .map(|entry| entry.expect("dir entry").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| cmrt_core::is_vvp_patch_path(name))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "`.vvp` が 1 件も無い: {dir:?}");

    let plugin = CatalogPlugin {
        name: "Vaporizer2".to_string(),
        plugin_path: "VASTvaporizer2.clap".to_string(),
        plugin_id: Some(cmrt_runtime::VAPORIZER2_PLUGIN_ID.to_string()),
        base: Some(root.to_string_lossy().into_owned()),
        dirs: vec![root.to_string_lossy().into_owned()],
        resolved_patches: None,
        source_notices: Vec::new(),
        patch_roles: cmrt_runtime::PatchRoles::default(),
    };
    let voicings = VvpVoicings::default();

    let mut undecided: Vec<&String> = Vec::new();
    let mut mono = 0usize;
    let mut poly = 0usize;
    // 起動時の先読みにかかる実時間。Stage 8 のベースラインとして print する。
    // 対照側の走査は別ループにして、この計測へ混ぜない。
    let started = std::time::Instant::now();
    for name in &names {
        match voicings.voicing(&plugin, name) {
            Some(PatchVoicing::Mono) => mono += 1,
            Some(PatchVoicing::Poly) => poly += 1,
            Some(PatchVoicing::Unknown) | None => undecided.push(name),
        }
    }
    let elapsed = started.elapsed();

    // 別経路の数え直し。`read_vvp_header` のパーサを通さず、もっと広い範囲を
    // 素朴に走査する。片方だけ壊れたときに気づけるよう 2 通りで数える。
    let mono_by_plain_scan = names
        .iter()
        .filter(|name| plain_scan_says_mono(&root.join(name)))
        .count();

    eprintln!(
        "`.vvp` {} 件 — poly {poly} / mono {mono} / 未判定 {} / 先読み {} ms",
        names.len(),
        undecided.len(),
        elapsed.as_millis()
    );
    assert!(
        undecided.is_empty(),
        "mono/poly を読めなかった音色がある: {undecided:?}"
    );
    assert_eq!(
        mono, mono_by_plain_scan,
        "ヘッダのパーサと素朴な走査で Mono の件数が食い違う"
    );
    // 全部 poly へ倒れていないこと（倒れると Mono が和音行へ出る）。
    assert!(mono > 0 && poly > 0, "mono {mono} / poly {poly}");
}

/// `read_vvp_header` を通さずに `m_uPolyMode` を読む。範囲も広く取る
/// （4096 バイトで足りているかを、この対照側では前提にしない）。
fn plain_scan_says_mono(path: &Path) -> bool {
    let mut file = std::fs::File::open(path).expect("音色ファイルを開けない");
    let mut prefix = vec![0u8; 64 * 1024];
    let read = std::io::Read::read(&mut file, &mut prefix).expect("音色ファイルを読めない");
    let text = String::from_utf8_lossy(&prefix[..read]).into_owned();
    let at = text
        .find("m_uPolyMode")
        .unwrap_or_else(|| panic!("m_uPolyMode が無い: {}", path.display()));
    text[at..]
        .split_once("/>")
        .map(|(element, _)| element.contains("text=\"Mono\""))
        .unwrap_or(false)
}
