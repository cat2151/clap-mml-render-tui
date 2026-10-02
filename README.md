# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Easily enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) / [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html) / [Shu](https://mikey.audio/shu) / [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) with MML. Written in Rust.

### Usage

- For playing around with MML sounds
- For casual installation. Just having Rust is enough.

### Tech Stack
- Plugin host library
  - https://github.com/prokopyl/clack

### Setup

Please install [Surge XT](https://surge-synthesizer.github.io/)

```
winget install "Surge XT"
```

### Install

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
```

### Run

```
cmrt
```

You can enter MML and play around in the TUI screen.

#### Play Server Implementation

Sound playback is handled by a separate process, the play server. Its executable is determined in the following order, and the first one found is used:

1. The full path specified by `--play-server <PATH>` (if the specified path does not exist, it will stop with an error instead of searching further)
2. `clap-mml-realtime-play-server` in the same directory as `cmrt`
3. The release build from the sibling repository (`../clap-mml-play-server/target/release/`)

PATH is not searched. Debug build servers have 4-5 times slower pre-loading, causing playback to cut off at the beginning of measures.
A warning will appear in the upper right corner of the screen when a debug build or an executable of unknown origin is being used.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- *Limited to CLAP plugins available for free on Windows without account registration
- [Surge XT](https://surge-synthesizer.github.io/)
- [Dexed](https://asb2m10.github.io/dexed/)
- [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2)
- [Floe](https://floe.audio/)
- [Sforzando](https://www.plogue.com/products/sforzando.html)
- [Six Sines](https://github.com/baconpaul/six-sines)
- [TyrellN6](https://u-he.com/products/tyrelln6/)
- Effects (can be inserted in series into DAW mode tracks)
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects (bundled with Surge XT)
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
- SFZ (those that allow trying key switches and control changes on a dedicated screen)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 
- Download pages for each (for those who get lost)
  - Surge XT: Easy to get with winget: [Fastest! How to install DAW and audio plugins even for cats (up to playing sound with virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe Download Page](https://floe.audio/download/)
  - [Sforzando Download Page](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub Releases Page](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6 Download Page](https://u-he.com/products/tyrelln6/)
  - [TONE3000 Download Page](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub Releases Page](https://github.com/michaelwillis/dragonfly-reverb/releases)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 

### AI Generated Document
- The following sections added by AI are difficult to read. I will maintain them occasionally.

### Keyboard Screen

Press the `v` key to move to the keyboard screen.

- `c d e f g a b` keys: Play C-D-E-F-G-A-B (do-re-mi-fa-sol-la-si)

### Chord Chart Screen

Press `Ctrl+G` then `C` to navigate to the chord chart screen.

This screen allows you to overview and edit the "structure" of chord progressions for an entire song. Moving the cursor will play the chord progression of that row.
By stepping through chords one by one within a row using `h` `l`, only the selected chord will play.

- Left pane (Sections): Define named chord progressions that serve as building blocks
- Right pane (Arrangement): The sequence of defined sections forms the song. The same section can be arranged multiple times.
- If you modify a section's progression, all references to it in the song will change simultaneously.
- The single line in the header shows the chord2mml specification (`Key=C BPM120`, etc.) to be placed at the beginning of the song.
- Neither progressions nor headers are interpreted by this screen. It retains the entered strings as-is.
- Only the section on the cursor's row plays (not the entire song). You can choose the overall timbre for the Chord Chart from the progression editing screen.
- You can preview chords one by one within a row using `h` `l`. The currently selected chord appears inverted within the progression.
- Only the Key from the header is passed for preview (BPM remains at its default).
- In previews, inversions and octaves are always auto-voiced. In Sections, they are determined within the section; in Arrangement, by the overall song sequence. `h` `l` single-chord previews also maintain the same inversion.

```
┌ Chord Chart ─────────────────────────────────────────────────────────────────┐
│ Key=C BPM120                                                                 │
│┌ Sections ───────────────────────┐┌ Arrangement ────────────────────────────┐│
││> 1 Intro    I-V                 ││  1 Intro    I-V                         ││
││  2 A        I-V-VIm-IV          ││  2 A        I-V-VIm-IV                  ││
││  3 B        IIm-V-I-VIm         ││> 3 A        I-V-VIm-IV                  ││
││                                 ││  4 B        IIm-V-I-VIm                 ││
│└─────────────────────────────────┘└─────────────────────────────────────────┘│
│ q:終了 ?:help                                                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

Here are the keybindings. Only `q` and `?` are displayed on the bottom line of the screen. The full list appears on-screen with the `?` key.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Pane movement (toggle Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Cursor movement (row. The chord progression of the destination row will all play.) |
| `h` `l` `←` `→` | Common | Move within chords in a row (only the pointed chord plays. At the end of a row, it wraps to the next row.) |
| `PgUp` `PgDn` | Common | Move cursor 10 rows. |
| `dd` | Common | Delete cursor row (deleting in Sections also removes references in Arrangement). |
| `Alt+↑` `Alt+↓` | Common | Move cursor row up / down. |
| `b` | Common | Rewrite header Key / BPM with single line input. |
| `Shift+P` `Space` | Common | Preview the section on the cursor row (stops if already playing). |
| `?` | Common | Toggle help (closes with `Esc` as well). |
| `q` | Common | Quit application. |
| `g` | Sections | Add a section by drawing from the chord progression catalog. |
| `r` | Sections | Re-draw the progression for the cursor row, keeping its name. |
| `i` | Sections | Edit progression using a single-line MML overlay for Chord Chart. |
| `n` | Sections | Edit name with single-line input. |
| `1`〜`9` | Arrangement | Insert the section with that number after the cursor. |

The editing screen opened with `i` in Sections initializes with the current progression and places the cursor at the end.
While typing, the chord at the cursor position will play with auto-voicing each time it forms or changes. Pressing `Ctrl+Space` will play the entire progression with the same voicing. Unreadable input will not be played as alternative MML but can still be saved as-is.
Press `Enter` to confirm and save the progression, trimming leading/trailing spaces. Press `Esc` to discard changes.

Within the editing screen, `Ctrl+T` opens the same timbre list as the normal MML overlay. Moving through candidates alone will not change the timbre. Press `Enter` to confirm, or `Esc` to revert to the original timbre. The Chord Chart's timbre is global for the entire screen, and it is saved to `history.json` separately from the timbre of the normal `Ctrl+P` MML overlay. Even if you discard progression edits with `Esc` after confirming a timbre, the confirmed timbre remains.

It is automatically saved every time you edit. The save location is under the configuration directory:
`clap-mml-render-tui/history/chord_chart.json` (on Windows, it's
`%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time.
If the save file is missing or unreadable, the screen performs a single `g`-like lottery when first opened, starting with one section. (If no section can be drawn, it remains empty.)

The chord progression catalog used for the `g` / `r` lottery is fetched from the network. It will only be delayed the first time if the cache is not yet available (the waiting time is recorded in log.txt under:
`chord-chart: event=catalog-first-load elapsed_ms=...`). If it fails to fetch,
"No chord progression data" will appear at the bottom of the screen.

### DAW Screen Effect Chain

In DAW screen's NORMAL mode, placing the cursor on a performance track and pressing `x` opens the EFFECT CHAIN overlay for that track.
You can insert any number of TONE3000 / Surge XT Effects / Dragonfly Reverb factory presets in series after the instrument (timbre).

| Key | Action |
|---|---|
| `x` | Open EFFECT CHAIN overlay for the cursor track (invalid for chord rows and conductor rows). |
| `j` `k` | Move through chain stages. |
| `a` | Open the add overlay. All factory presets for all effects are listed in one column. Select with `j` `k`, press `Enter` to add to the end, `ESC` to go back. |
| `dd` | Delete the cursor's stage. |
| `Enter` | Write back to the init column's JSON and close (the track's cache WAV will be re-rendered). |
| `ESC` | Discard changes and close. |

- The chain is saved in the init column's JSON under `"effects after instrument"` (array order = signal order). Editing the init column directly has the same effect.
- Effects are baked into the cache WAV (applied on the render-server side).
- Each effect's preset is loaded from the built-in default locations (`%ProgramData%\TONE3000\Presets`, `%ProgramData%\Surge XT\fx_presets`). Dragonfly Reverb's presets are embedded in the plugin itself, so if the plugin exists at `C:\Program Files\Common Files\CLAP\dragonfly-reverb\`, it will appear as a candidate. If the plugin is not found, it won't appear.
- The chain runs only for the duration of the notes, so reverb tails will be cut off at the end of the cell.

In the Guitar Articulation screen (`Ctrl+G` → `E`), pressing `x` also opens the same EFFECT CHAIN overlay.
You can insert a guitar amp (`a` → kind `Amp Simulator`, TONE3000) or other effects after METAL-GTX and preview them with articulated playback each time you change a stage.
Confirming with `Enter` will apply that chain to subsequent `b` and `space` playback. The chain is discarded when the application exits.

### Configuration

On first launch, `config.toml` is automatically created. Its location is under the OS's standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. Restart the application after closing the editor.

Here is an example configuration:

```toml
# 【必須】使用する CLAP プラグイン
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# config.toml を開く editor 候補（左から順に試す）
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav は自動的に設定ディレクトリ配下の
# clap-mml-render-tui/phrase/ または clap-mml-render-tui/daw/ に保存されます。
# 以下の値は内部的に使用されます。
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# オフラインレンダリングは render-server 子プロセスで行います。
# 同時実行数（1〜16）・port・起動コマンド（空なら実体を探索）
offline_render_server_workers = 4
offline_render_server_port = 42153
offline_render_server_command = ""

# リアルタイム再生 backend（"cache_player" / "play_server"）
realtime_audio_backend = "cache_player"
realtime_play_server_port = 42154

# 起動時に自動再生するかどうか
# notepad モード: 現在行を即座に再生します。DAW モード: 曲先頭（measure 0）から演奏開始します。
autoplay_on_startup = true

# WAV ループブラウザーの検索対象ディレクトリ一覧
loop_dirs = []

# WAV ループディレクトリへ付与できるカテゴリ一覧
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]

# Surge XT の標準値を変える場合だけ書く
[plugins."Surge XT"]
patches_dirs = [
  'C:\ProgramData\Surge XT\patches_factory',
  'C:\ProgramData\Surge XT\patches_3rdparty',
]
```

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugins."Surge XT".plugin_path` | OS-specific Surge XT CLAP standard path | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent offline rendering (render-server) tasks. |
| `offline_render_server_port` | `42153` | Localhost port for the render-server. |
| `offline_render_server_command` | Empty string | Startup command for the render-server. If empty, the executable is searched for. |
| `realtime_audio_backend` | `cache_player` | Execution target for real-time playback (`cache_player` / `play_server`). |
| `realtime_play_server_port` | `42154` | Localhost port for the play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific Surge XT patches standard directories | List of directories to search for Surge XT timbres. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. Run `cmrt scan-loops` after making changes. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. Category overlay keys are determined from unused English letters in category names. |

The default `plugin_path` for each OS is as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The default `patches_dirs` for each OS is as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines without a specified timbre is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to a mixed catalog via `[plugins.<name>]` and used on lines where their timbre is explicitly stated.

The contents of the built-in profiles are as follows, with paths being the standard installation locations for each OS:

| Name | `plugin_id` | `patches_dirs` | Category by Usage |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Surge XT category name |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | `Presets` under registry (`HKLM\SOFTWARE\VAST Dynamics\Vaporizer2\Settings`'s `InstallPath`). `patches_dirs` in config is ignored. | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio`, etc.) |
| `Six Sines` | `org.baconpaul.six-sines` | `%LOCALAPPDATA%\clap-mml-render-tui\vendor-patches\six-sines-factory` (factory timbre acquisition source). `patches_dirs` in config is ignored. | All empty (= no filtering) |
| `TyrellN6` | `com.u-he.TyrellN6` | `Presets\TyrellN6` under registry (`HKCU\Software\u-he\TyrellN6`'s `DataPath`). `patches_dirs` in config is ignored. | All empty (= no filtering) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated the same).

You should only use `[plugins.<name>]` if you've installed a plugin in a non-standard location, need to specify a timbre location, or are using a plugin not built-in. **Only the specified items will override built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within that table is sufficient.

```toml
# パスだけ差し替える。plugin_id と patches_dirs は組み込みの値のまま。
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# 組み込みに無いプラグインは全部書く。
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<name>.plugin_path` | Path to that plugin. |
| `plugins.<name>.plugin_id` | Expected CLAP plugin ID. Optional. |
| `plugins.<name>.patches_dirs` | Location of timbres for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<usage>_patch_categories` / `<role>_patch_keywords` | Filtering for usage-specific patch auto-selection. Seven key names can be written: (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the written items apply to that plugin. If not written, the plugin's default value (Surge XT uses category names, others use "no filtering") will be used. |

- `active_plugin` is deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific category items at the top level will result in a configuration error, rather than being silently ignored. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default values; others become candidates in the mixed catalog.
- Dexed timbres are "1 `.syx` cartridge = 32 programs", so in the list, each program is displayed individually, treating the cartridge as a directory, e.g., `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2-digits, starting from 0). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not a timbre setting, and its default is POLY. Therefore, all Dexed timbres are treated as chord-oriented in the grid sequencer's chord rows.
- Vaporizer2 timbres are 1 `.vvp` file = 1 timbre, and can be selected just like Surge's `.fxp` files. The category shown in the list's heading is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Vaporizer2 and Floe timbre locations are read from the plugin's own settings (Vaporizer2 from `InstallPath\Presets` in the registry; Floe from `extra-presets-folder` in `%PUBLIC%\Floe\Preferences\floe.ini`, or `%PUBLIC%\Floe\Presets` if unset for Floe). Reinstallation or plugin-side setting changes do not require rewriting config.toml. `patches_dirs` entries under `[plugins.Vaporizer2]` / `[plugins.Floe]` will be ignored.
- Six Sines factory timbres are embedded in the plugin itself and not on disk. Therefore, `cmrt build-patch-catalog-cache` fetches the same version as the installed Six Sines from GitHub (if already fetched and version is the same, no communication occurs). Mono/poly is read from the `.sxsnp` content (play mode).
- TyrellN6 timbres are 1 `.h2p` file = 1 timbre, and the subfolders (`01 Basses`, etc.) under the location become categories in the list. `UserPresets` are not enumerated. Mono/poly is not read from the timbre; all timbres are treated as chord-oriented in the grid sequencer's chord rows.

- Vaporizer2's mono/poly differs per timbre and is read from the `.vvp` content (`m_uPolyMode`). Therefore, only timbres that play chords are listed as candidates in the grid sequencer's chord rows (unreadable timbres are not presented as chord row candidates).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in cmrt. These timbres are designed to work with MPE (per-note pitch and pressure) performance data, which cmrt does not send.
- The default category settings for filtering candidates by row usage (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (meaning all programs are candidates for any row). This is because Dexed cartridges are not organized by "directory name = usage," and the timbre organization for non-built-in plugins is unknown. If you wish to change this, please write the 7 items under `[plugins.<name>]` (the default values for Surge XT are included as comments at the end of the generated config.toml).
- The 7 usage-specific category items should also only be written within the plugin profile. For Surge XT, place them under `[plugins."Surge XT"]`; for other plugins, place them in the plugin's own table.
- The shared mono/poly determination data (`voicing_shared_source` / `voicing_override_source`) used for usage-specific auto-selection is only for Surge XT timbre determination.
- Rendering result caches are placed in separate directories per plugin, so even if mixed, sounds from different plugins will not be misused (no manual deletion required). There are two locations: `<plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAV)

All offline rendering is done via the render-server. The TUI side (`cmrt.exe`) does not load CLAP plugins, but instead sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, cmrt starts a child process, and in case of a communication error, it restarts and retries once. If `offline_render_server_command` is empty, the child process executable is searched for in the following order: "same directory as `cmrt.exe`" -> "release build of sibling repo `clap-mml-play-server`". **PATH is not searched.**

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works in conjunction with the bluesky-text-to-audio Chrome extension
  - When an MML is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing "cde" will play C-D-E.

```
cmrt CM7
```

- Typing "CM7" will play a C major seventh chord.
- It also supports various chord progression notations (some are not yet supported).

### Patch Roles Command

```
cmrt patch-roles
```

- For each row in the grid sequencer (chord / bass / arpeggio / 4 drum roles / other), the PATCH column
  displays how many timbre candidates are available via the wheel. The screen does not launch.
- After changing plugins, `patches_dirs`, or usage-specific categories (`chord_patch_categories`, etc.),
  use this to check if "turning the wheel yields no response".
- If any row has 0 candidates, it lists that row and exits with exit code 1.
- Adding `--config <path>` reads that config.toml. This allows you to test what happens if settings are changed
  without modifying your currently used config.toml.
- When multiple plugin timbres are listed, the count of candidates per usage will also include a plugin-specific breakdown.
  This is because relying solely on the total count might prevent noticing if "a certain plugin's timbre is not appearing at all for that row."

```
cmrt patch-roles --config C:\tmp\try.toml
```

### Render MML Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified timbre and displays its length, volume (`peak` / `rms`), whether it's silent, and
  a digest value of the output sound on a single line. The screen does not launch.
- `--patch` can be specified multiple times. A summary line "N / M distinct sounds" will appear, so you can check
  if **the sound remains the same despite changing the timbre**.
- Adding `--out-dir <directory>` writes WAV files (if not added, not a single byte is written).
  Use this when you want to confirm by ear.
- Adding `--poly-check` compares playing chords and single notes to determine if the timbre can play chords.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It's more logical to obtain Surge XT patches via an API, so that will be implemented (currently it searches paths specified in toml, which is inefficient. Implementation timing is deferred; other priorities come first).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in 1-measure units,"
    - while introducing constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - *Leaving "アトミック小節" (Atomic Measure) untranslated for now, as "atomic measure" sounds like a physics term in English.

# Out of Scope
- Effects are essential for editing, so they are intentionally deemed out of scope and deferred to a much later stage. One reason for this is that Surge XT's patches include effects (effects are derived from patches).