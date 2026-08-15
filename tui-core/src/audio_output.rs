//! TUI 内で使う rodio 出力の生成ポリシー。

/// 端末を直接書き換える Drop ログを無効化した既定オーディオ出力を開く。
///
/// rodio 0.22 の `MixerDeviceSink` は既定で Drop 時に stderr へ出力する。
/// alternate screen 中の stderr は ratatui の差分bufferを壊すため、TUI から開く
/// DeviceSink は必ずこの関数を経由させる。
pub fn open_default_sink() -> Result<rodio::MixerDeviceSink, rodio::DeviceSinkError> {
    let mut sink = rodio::DeviceSinkBuilder::open_default_sink()?;
    sink.log_on_drop(false);
    Ok(sink)
}
