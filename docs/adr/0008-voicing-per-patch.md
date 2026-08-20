# ADR 0008: voicing は patch ごとに引く / 未知プラグインは poly とみなす

- 状態: 採用（2026-08-20）
- 関連: [0007](0007-patch-role-defaults-three-layers.md) /
  play-server `docs/adr/0005-dexed-mono-mode-is-poly.md`

## 決定

`VoicingPolicy`（`app/src/tui/voicing.rs`）は Config 全体に 1 つではなく、
**`VoicingPolicies` として patch ごとに引き分ける。**

| 値 | 対象 | 振る舞い |
|---|---|---|
| `Sources` | Surge XT | shared JSON / ユーザー判定 / override の 3 層で引く。未判定は `None` |
| `AssumePoly` | それ以外 | **全 patch を poly とみなす** |

## なぜ `AssumePoly` を「判定不能」にしなかったか

`matches_role` は `PatchRole::Chord` で `is_poly` を要求し、**未判定は外れ扱い**になる。
`None` を返すと**和音行の候補が必ず 0 件になり、画面ごと使えなくなる**。

poly と外した場合の実害（和音行で単音の音色が鳴る）のほうが軽いので poly 側へ倒した。
Dexed については実測（play-server 側 ADR）により「外した推測」ではない。

## Surge 専用リソースは他プラグインで取りに行かない

`voicing_shared_source` / `voicing_override_source` は `SourceSet::from_config()` が
`!cfg.is_surge_xt()` で `None` を返すので**ダウンロードもキャッシュ読みも発生しない**。

混在後は判定が「既定プラグインが Surge か」→「**カタログに Surge が載るか**」へ変わった。
`cmrt build-voicing-cache` は probe できない音色（cartridge）を対象から外す。

## 罠

- **`AssumePoly` は「判定していない」ではなく「poly と決めた」。**
  keyboard のステータス行も `detect: poly`（Cached）になる。
  Dexed では実測どおりだが、**未知のプラグインでは推測である**
- 判別材料は patch 文字列の形だけなので、**同じ形を扱うプラグインが 2 つ載ると区別できない**
- MIDI dialect には `note_id` が無いので `NoteEnd` が返らない。voicing probe は dialect が
  CLAP を含まない場合は**実行しない**（詳細は play-server 側 ADR）
