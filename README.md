# clap-mml-render-tui

## Overview
MML TUI DAW (or similar). Easily enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) / [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html) / [Shu](https://mikey.audio/shu) / [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) using MML. Written in Rust.

## Usage

- For playing around with MML sound
- For casual installation. Just having Rust is enough

## Technology Stack
- Plugin host library
  - https://github.com/prokopyl/clack

## Setup

Please install [Surge XT](https://surge-synthesizer.github.io/)

```
winget install "Surge XT"
```

## Installation

```
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
```

## Run

```
cmrt
```

You can input MML and play around in the TUI screen.

## Patch Catalog

You can create a patch catalog for Dexed and other plugins, and use it for filtering, etc.

```
cmrt build-patch-catalog-cache
```

## Supported Audio Plugins
- Note: Limited to plugins that are CLAP, Windows compatible, and available for free without account registration.
- [Surge XT](https://surge-synthesizer.github.io/)
- [Dexed](https://asb2m10.github.io/dexed/)
- [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2)
- [Floe](https://floe.audio/)
- [Sforzando](https://www.plogue.com/products/sforzando.html)
- [Six Sines](https://github.com/baconpaul/six-sines)
- [TyrellN6](https://u-he.com/products/tyrelln6/)
- effect (can be inserted in series to tracks in DAW mode)
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects (bundled with Surge XT)
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
- sfz (those that allow trying key switches and control changes on a dedicated screen)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx)
- Each download screen (for those who got lost)
  - Surge XT: Easy to get with winget: [Fastest! How to install DAW and audio plugins even for cats (until you make sound with a virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe Download page](https://floe.audio/download/)
  - [Sforzando Download page](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub releases page](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6 Download page](https://u-he.com/products/tyrelln6/)
  - [TONE3000 Download page](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub releases page](https://github.com/michaelwillis/dragonfly-reverb/releases)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx)

## AI-Generated Documentation
- The following sections, added by AI, are currently difficult to read. I will maintain them occasionally.
- AI, please write only in this section. Do not write in other sections.

### Play Server Implementation

The sound is played by a separate process, the play server. Its executable is determined in the following order, and the first one found is used:

1. The full path specified by `--play-server <PATH>` (if not found, it stops with an error without searching further)
2. `clap-mml-realtime-play-server` in the same directory as `cmrt`
3. The release build of the sibling repository (`../clap-mml-play-server/target/release/`)

PATH is not checked. Debug build servers have a 4-5 times slower pre-buffering, causing playback to cut off at the beginning of measures.
A warning appears in the upper right corner of the screen when a debug build or an unknown executable is being used.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Keyboard Screen

Press the `v` key to move to the keyboard screen.

- `c d e f g a b` keys: Play C D E F G A B.

### Chord Chart Screen

Press `Ctrl+G` then `C` to move to the chord chart screen.

This screen allows you to overview and edit the "composition" of a song's chord progression. Moving the cursor plays the chord progression of that line.
When you traverse the chords within a line one by one using `h` `l`, only the pointed chord plays.

- Left pane (Sections): Define chord progressions as material with names.
- Right pane (Arrangement): The sequence of defined sections becomes the song. The same section can be arranged multiple times.
- If you modify a section's progression, all references to it in the song will change together.
- The first line of the header directly displays the chord2mml specification (`Key=C BPM120`, etc.) placed at the beginning of the song.
- Neither the progression nor the header is interpreted by this screen. It retains the typed string as-is.
- Only one section on the cursor line plays (not the entire song). You can select the overall timbre for the Chord Chart from the progression editing screen.
- You can audition chords one by one within a line using `h` `l`. The currently pointed chord appears inverted within the progression.
- Only the Key from the header is passed to the audition (BPM remains at its default).
- Audition inversions and octaves are always auto-voiced. In Sections, this is determined within the section, in Arrangement, from the overall song order, and the same voicing is maintained even during `h` `l` single chord auditions.

```
┌ Chord Chart ─────────────────────────────────────────────────────────────────┐
│ Key=C BPM120                                                                 │
│┌ Sections ───────────────────────┐┌ Arrangement ────────────────────────────┐│
││> 1 Intro    I-V                 ││  1 Intro    I-V                         ││
││  2 A        I-V-VIm-IV          ││  2 A        I-V-VIm-IV                  ││
││  3 B        IIm-V-I-VIm         ││> 3 A        I-V-VIm-IV                  ││
││                                 ││  4 B        IIm-V-I-VIm                 ││
│└─────────────────────────────────┘└─────────────────────────────────────────┘│
│ q:Quit ?:help                                                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

Here are the keybindings. Only `q` and `?` are displayed on the bottom line of the screen. The full list appears on-screen when pressing `?`.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Pane movement (toggle Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Cursor movement (line. The chord progression of the destination line plays entirely) |
| `h` `l` `←` `→` | Common | Chord movement within a line (only the pointed chord plays. Wraps to the adjacent line at the end of a line) |
| `PgUp` `PgDn` | Common | Move cursor 10 lines |
| `dd` | Common | Delete cursor line (deleting from Sections also removes its references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Move cursor line up / down |
| `b` | Common | Rewrite header Key / BPM with single-line input |
| `Shift+P` `Space` | Common | Audition the section on the cursor line (stops if already playing) |
| `x` | Common | Select effect chain for Chord timbre (confirm with `Enter`, discard with `Esc`. Does not apply to Bass timbre) |
| `?` | Common | Open/close help (also closes with `Esc`) |
| `q` | Common | Quit application |
| `g` | Sections | Add a section by drawing from the chord progression catalog |
| `r` | Sections | Re-draw the progression of the cursor line, preserving its name |
| `i` | Sections | Edit progression with single-line MML overlay for Chord Chart |
| `n` | Sections | Edit name with single-line input |
| `1`〜`9` | Arrangement | Insert the section with that number after the cursor |

The editing screen opened by `i` in Sections initializes with the current progression and places the cursor at the end.
While typing, the chord at the cursor position will auto-voice and play as it forms or changes. Press `Ctrl+Space` to play the entire progression with the same voicing. Unreadable input will not be played as MML but can be saved as is. Press `Enter` to confirm and save the progression (after trimming leading/trailing whitespace), or `Esc` to discard changes.

Inside the editing screen, `Ctrl+T` opens the same patch list as the regular MML overlay. Moving through candidates alone does not change the patch; `Enter` confirms, `Esc` reverts to the original patch. The Chord Chart's patch is a single global setting for the entire screen and is saved to `history.json` separately from the regular `Ctrl+P` MML overlay patch. If you discard the progression edit with `Esc` after confirming a patch, the confirmed patch remains.

Changes are automatically saved each time you edit. The save location is `clap-mml-render-tui/history/chord_chart.json` within the configuration directory (on Windows, this is `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time.
If the save file is missing or unreadable, when the screen is first opened, `g` will perform a single lottery drawing to start with one section (if a section cannot be drawn, it remains empty).

The chord progression catalog used for `g` / `r` lottery is retrieved from the network. Only the first time, when the cache is not yet available, will there be a wait (the waiting time is recorded in log.txt under `chord-chart: event=catalog-first-load elapsed_ms=...`). If the data cannot be retrieved, "Chord progression data not available" will appear at the bottom of the screen.

### Effect Chain in DAW Screen

In NORMAL mode of the DAW screen, pressing `x` while the cursor is on a playback track opens that track's EFFECT CHAIN overlay.
You can insert any number of TONE3000 / Surge XT Effects / Dragonfly Reverb factory presets in series after the instrument (patch).

| Key | Action |
|---|---|
| `x` | Opens the EFFECT CHAIN overlay for the cursor track (invalid for chord or conductor lines) |
| `j` `k` | Move along the chain's stages |
| `a` | Opens the add overlay. All factory presets of all effects are listed in one column. Select with `j` `k`, add to the end with `Enter`, and return with `ESC` |
| `dd` | Delete the stage at the cursor |
| `Enter` | Writes back to the init column's JSON and closes (the track's cached WAV is re-rendered) |
| `ESC` | Discard changes and close |

- The chain is saved in the init column's JSON under `"effects after instrument"` (array order = signal order). Editing the init column directly has the same effect.
- Effects are baked into the cached WAV (applied on the render-server side).
- Each effect's preset is loaded from its built-in default location (`%ProgramData%\TONE3000\Presets`, `%ProgramData%\Surge XT\fx_presets`). Dragonfly Reverb presets are built into the plugin itself, so if the plugin is present at `C:\Program Files\Common Files\CLAP\dragonfly-reverb\`, it will appear as a candidate. If the plugin is not found, it won't appear.
- The chain runs for the same duration as the note, so reverb tails are cut off at the end of the cell.

In the Guitar Articulation screen (`Ctrl+G` → `E`), pressing `x` also opens the same EFFECT CHAIN overlay.
You can insert a guitar amp (`a` → kind `Amp Simulator`, TONE3000) or other effects after METAL-GTX, and audition the articulated performance each time you change a stage.
Confirming with `Enter` applies that chain to subsequent `b` and `space` performances. The chain disappears when the application is closed.

### Configuration

A `config.toml` file is automatically created on first launch. Its location is within the OS-standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In NORMAL mode of TUI / DAW, pressing `e` opens `config.toml` with an editor. After closing the editor, restart the application.

Here is a current configuration example:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved to
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/
# within the configuration directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Offline rendering is performed by render-server child processes.
# Concurrent execution count (1-16)・port・startup command (if empty, searches for executable)
offline_render_server_workers = 4
offline_render_server_port = 42153
offline_render_server_command = ""

# Real-time playback backend ("cache_player" / "play_server")
realtime_audio_backend = "cache_player"
realtime_play_server_port = 42154

# Whether to autoplay on startup
# Notepad mode: Immediately plays the current line. DAW mode: Starts playback from the beginning of the song (measure 0).
autoplay_on_startup = true

# List of directories to search for in the WAV loop browser
loop_dirs = []

# List of categories that can be assigned to WAV loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]

# Only write if changing Surge XT's default values
[plugins."Surge XT"]
patches_dirs = [
  'C:\ProgramData\Surge XT\patches_factory',
  'C:\ProgramData\Surge XT\patches_3rdparty',
]
```

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugins."Surge XT".plugin_path` | OS-specific standard path for Surge XT CLAP | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent executions for offline rendering (render-server). |
| `offline_render_server_port` | `42153` | Localhost port for the render-server. |
| `offline_render_server_command` | Empty string | Startup command for the render-server. If empty, it searches for the executable. |
| `realtime_audio_backend` | `cache_player` | Target for real-time playback (`cache_player` / `play_server`). |
| `realtime_play_server_port` | `42154` | Localhost port for the play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific standard directories for Surge XT patches | List of directories to search for Surge XT patches. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. The key for the category overlay is determined from unused English letters in the category name. |

OS-specific default values for `plugin_path` are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS-specific default values for `patches_dirs` are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin that plays when no patch is specified is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to the mixed catalog from `[plugins.<name>]` and used on lines where a patch is explicitly stated.

The contents of the built-in profiles are as follows, with paths being the OS-specific standard installation locations:

| Name | plugin_id | patches_dirs | Category by Usage |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed's cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | `Presets` under the registry's `InstallPath` (`HKLM\SOFTWARE\VAST Dynamics\Vaporizer2\Settings`). `patches_dirs` in config is ignored | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio`, etc.) |
| `Six Sines` | `org.baconpaul.six-sines` | `%LOCALAPPDATA%\clap-mml-render-tui\vendor-patches\six-sines-factory` (factory patch acquisition location). `patches_dirs` in config is ignored | All empty (= no filtering) |
| `TyrellN6` | `com.u-he.TyrellN6` | `Presets\TyrellN6` under the registry's `DataPath` (`HKCU\Software\u-he\TyrellN6`). `patches_dirs` in config is ignored | All empty (= no filtering) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

Only write `[plugins.<name>]` if you are installing in a non-standard location, supplementing patch locations, or using plugins not built-in. **Only the written items will override the built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within that table is sufficient.

```toml
# Only override the path. plugin_id and patches_dirs remain at their built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, write all properties.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<name>.plugin_path` | Path to that plugin. |
| `plugins.<name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | Patch locations for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<usage>_patch_categories` / `<role>_patch_keywords` | Filtering for automatic patch selection by usage. Seven key names can be written (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the written items take effect for that plugin. If not written, the plugin's default value (Surge XT uses category names, others use "no filtering") is used. |

- `active_plugin` has been deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific category items at the top level will result in a configuration error rather than being silently ignored. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` does not change the default plugin. Only a profile with the same name as Surge XT acts as an override for the fixed default, while others become candidates in the mixed catalog.
- Dexed patches are '1 `.syx` cartridge = 32 programs', so in the list, cartridges are treated as directories, and programs are listed individually as `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 0-indexed, two digits). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not a patch property, and its default is POLY. Therefore, all Dexed patches are treated as suitable for chords in the grid sequencer's chord lines.
- Vaporizer2 patches are '1 `.vvp` file = 1 patch', and can be selected just like Surge's `.fxp` files. The category that appears in the list's heading is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Vaporizer2 and Floe's patch locations are read from the plugin's own settings (Vaporizer2 from the registry's `InstallPath\Presets`, Floe from `%PUBLIC%\Floe\Preferences\floe.ini`'s `extra-presets-folder`; if unset for Floe, `%PUBLIC%\Floe\Presets`). Reinstallation or changes to the plugin's settings do not require rewriting `config.toml`. `patches_dirs` written in `[plugins.Vaporizer2]` / `[plugins.Floe]` will be ignored.
- Six Sines' factory patches are embedded within the plugin itself and not on disk. Therefore, during `cmrt build-patch-catalog-cache`, they are retrieved from GitHub if they are the same version as the installed Six Sines (no communication if already retrieved and same version). Mono/poly is read from the `.sxsnp` content (play mode).
- TyrellN6 patches are '1 `.h2p` file = 1 patch', and the subfolders under its location (e.g., `01 Basses`) become categories in the list. `UserPresets` are not enumerated. Mono/poly is not read from the patch; all patches are treated as suitable for chords in the grid sequencer's chord lines.

- Vaporizer2's mono/poly varies per patch and is read from the `.vvp` content (`m_uPolyMode`). Therefore, only patches that play chords are presented as candidates in the grid sequencer's chord lines (unreadable patches are not listed as candidates for chord lines).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in cmrt. These patches are designed to be played with MPE (per-note pitch and pressure) performance information, which cmrt does not send.
- The default category settings for filtering candidates by line usage (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (= all programs are candidates for all lines). This is because Dexed cartridges do not have "directory name = usage" and non-built-in plugins' patch organization is unknown, so they are not filtered. If you want to change this, please write the 7 items in `[plugins.<name>]` (the default values for Surge XT are included as comments at the end of the generated `config.toml`).
- The 7 usage-specific category items should also only be written within the plugin profile. For Surge XT, use `[plugins."Surge XT"]`; for other plugins, place them in that plugin's own table.
- The shared determination data for mono/poly (`voicing_shared_source` / `voicing_override_source`) used for usage-specific auto-selection is only applied to Surge XT patch determination.
- Among the patches in the drum percussion column, those that empirically show pitch tracking when the note is changed (where `tracks_keyboard` is true in the shared table `cat-music-patterns/patch-pitch-tracking.json`) are moved to the lead's `pitched percussion` column, regardless of the plugin (patches whose names indicate mallet or bell are moved to that column). Patches in the kick, snare, and hi-hat columns are not moved. The table is retrieved on startup and saved to `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\patch-pitch-tracking.json`. If it cannot be retrieved, the previously saved table is used; if that is also unavailable, the list is displayed without moving patches. The table can be rebuilt with `cmrt build-pitch-tracking --output <PATH>` (this measures each patch by launching `clap-mml-play-server`'s `probe-pitch-tracking`).
- The rendering result cache is placed in a separate directory for each plugin, so mixing plugins will not lead to incorrect use of another plugin's sound (no manual deletion is necessary). The locations are as follows, where `<plugin>` is the resolved `plugin_path` filename (without extension) (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAV)

All offline rendering is done via the render-server. The TUI side (`cmrt.exe`) does not load CLAP plugins; instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, cmrt starts a child process, and in case of communication error, it restarts and retries once. The child process executable is searched for in the following order if `offline_render_server_command` is empty: 'same directory as `cmrt.exe` -> release build of sibling repo `clap-mml-play-server`', and **PATH is not checked**.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Interoperates with the bluesky-text-to-audio Chrome extension.
  - When an MML snippet is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` plays C D E.

```
cmrt CM7
```

- Typing `CM7` plays C Major 7th.
- Also supports various chord progression notations (some are not yet supported).

### Patch-Roles Command

```
cmrt patch-roles
```

- Displays how many patch candidates are available for selection with the wheel in the PATCH column for each line of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). No screen is launched.
- Use this to check if the wheel becomes unresponsive after changing plugins, `patches_dirs`, or usage-specific categories (e.g., `chord_patch_categories`).
- If any line has 0 candidates, it lists that line and exits with code 1.
- Adding `--config <path>` reads that `config.toml`. This allows testing how changes affect the configuration without modifying the currently used `config.toml`.
- When patches from multiple plugins are listed, the breakdown by plugin is also shown for each usage's candidate count. This helps to notice if "a specific plugin's patch is not appearing on that line" which might be missed by just the total.

```
cmrt patch-roles --config C:\tmp\try.toml
```

### Render-MML Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified patch and displays length, volume (`peak` / `rms`), whether it's silent, and an output sound digest value on one line. No screen is launched.
- While `patch-roles` counts whether a patch appears in the list, this command checks whether that patch actually produces sound.
- Multiple `--patch` arguments can be chained. A summary line shows "N / M different outputs", which helps to determine if **the patch was changed but the sound remained the same as before**.
- Adding `--out-dir <directory>` writes WAV files (if not added, no bytes are written). Use this when you want to confirm by ear.
- Adding `--poly-check` compares playing chords and single notes to determine if the patch can play polyphonically.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

The above section was very difficult to read due to mixed AI-generated text. It will be revised later.

AI, please do not write outside of the sections mentioned above.

## Breaking Changes
- Frequent breaking changes occur daily.

## Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in 1-measure units",
    - While subject to certain constraints,
    - Various benefits can be gained.
    - This is suitable for sketching purposes and quickly iterating through editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - *Note: "atomic measure" might be confused with a physics term, so for now, we'll keep it as "アトミック小節" (Atomic Measure) without further translation.
- Experiments
    - Various experiments will be implemented.
- Personal Use
    - Frequent breaking changes will occur.
    - It is commonplace for things that worked yesterday not to work today.

## Out of Scope
- Practicality

## License
- MIT