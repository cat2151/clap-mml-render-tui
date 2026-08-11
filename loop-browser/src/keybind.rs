use crate::LoopBrowserPane;

/// loop browser 各ペインのキーバインド説明文（app のステータスバーからも参照）。
pub fn loop_browser_keybind_text(pane: LoopBrowserPane) -> &'static str {
    match pane {
        LoopBrowserPane::Tree => {
            "Ctrl+G:画面切替 Ctrl+B:BPM Tab:track list p:演奏停止/再開 r:ランダムWAV S+O:auto Shift+C/D/E/F/G/A/B:pad登録/解除 c/d/e/f/g/a/b:pad演奏 1-9:hjkl prefix PgUp/PgDn:±10 t:dirカテゴリ v:dirお気に入り V:お気に入り限定 hjkl・矢印:移動/展開 Enter/Space:再生 q:終了"
        }
        LoopBrowserPane::Tracks => {
            "Ctrl+G:画面切替 Ctrl+B:BPM Tab:loop tree p:演奏停止/再開 r:ランダムWAV m:mix level Shift+R:全track random S+O:auto Shift+M:2track random solo Alt+↓/↑:track並び替え 1-9:hjkl prefix s:solo toggle c..b:pad h/l・←/→:measure j/k・↓/↑:track（右/下端で追加） q:終了"
        }
    }
}
