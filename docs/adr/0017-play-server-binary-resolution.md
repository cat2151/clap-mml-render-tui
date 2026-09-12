# ADR 0017: play server の実体は探索順で決める（PATH 解決は廃止する）

- 状態: 採用（実装は `realtime-play/src/server_binary.rs`）
- 関連: [0010](0010-two-repo-layout.md) / [0016](0016-daw-live-playback-slots-and-timeline.md)

## 何の話か

[0016](0016-daw-live-playback-slots-and-timeline.md) で「ぶつ切り」を数値まで潰したあと、
**同じ症状が再発した。原因はコードではなく、debug ビルドの play server で起動していたこと。**
release で起動し直したら消えた。

debug サーバーは先読み 1 小節ぶんの state load（`next_ms`）が小節の **23%**（release は **5%**）かかり、
`preload=miss` が半分の小節で出て、境界で載せ直す 400ms 強がそのまま小節の頭の無音になる。
これが「ぶつ切り」の実体。

**どちらのサーバーが動いているかは、ログの `realtime-play: action=server-spawned ... fullpath=`
にしか出ていなかった。** 画面からは一切分からず、症状から原因へ辿り着くのにログの発掘が要った。

## 決定

### 実体の決め方は 3 段。上から順に、最初に見つかったものを使う

1. **コマンドライン引数で渡された fullpath**（`cmrt --play-server <PATH>`）
2. **`cmrt.exe` と同じディレクトリ**の `clap-mml-realtime-play-server.exe`
3. **兄弟 repo の release**
   （`<この repo の親>/clap-mml-play-server/target/release/clap-mml-realtime-play-server.exe`）

どれも見つからなければ、**探した場所が分かるエラーで止める。**
素の実行ファイル名で spawn して OS のエラーに任せる形（`source=unresolved-PATH`）は残さない。

1 番で指定した実体が存在しないときは、**探索へ落とさずそこで止める。**
打った指定が黙って無視されるのは、この ADR が潰した事故と同じ手触りになる。

3 番が効くのは `cmrt.exe` が `target/debug` か `target/release` の直下に居るときだけ。
つまり開発ビルドのときにしか成立せず、配布物では必ず 2 番で決まる。
ここを緩めると、ユーザーの PC のどこか上位に同名フォルダがあるだけで知らない実体を掴む。

### PATH 解決は廃止する。復活させない

事故の原因がここ。起動用 bat が PATH の先頭を切り替えるだけで debug / release が決まり、
**アプリも画面もそれを知らなかった。** 起動元の環境に依存して静かに別のバイナリを掴むのが事故そのもの。

### config の shell command（`Config::realtime_play_server_command`）も廃止する

「実体を環境側の設定が黙って決める」という点で PATH とまったく同じ形なので、一緒に落とす。
古い config.toml にキーが残っていても読めるが、**実体の決め方には一切効かない**。

廃止して困ったのは 1 点だけで、**テストが偽サーバー（`echo Error: boom 1>&2& exit 3` /
`exit 0`）を立てるのに shell を使っていた**。これは fullpath 指定では書けない。
そこで `Config` に **toml からは設定できない注入口**（`#[serde(skip)]` の
`play_server_launch_override: Option<PlayServerLaunch>`）を置き、
`--play-server` とテストの両方がそこへ載る形にした。
ユーザーがこれを設定する入口は存在しない。

### toml で profile を設定する案は不採用

コマンドライン引数（fullpath）と上の探索順で足りる。「debug か release か」という 2 値ではなく
**実体そのものを指定する**ほうが迷いようがない。

### 選ばれた実体と profile を画面に出す

判定は **4 値**。上から順に見て、最初に当てはまったものになる。

| profile | 何のとき | 画面 |
|---|---|---|
| `debug` | パスが `target/debug` を含む | **点灯** |
| `release` | パスが `target/release` を含む | 静か |
| `同梱` | `cmrt.exe` と同じディレクトリにある（= 配布物の通常形） | 静か |
| `不明` | 上のどれでもない | **点灯** |

**パスの判定を「同梱」より先に見る**のは、禁止されている「`./target/debug` へ手で cp」を
やってしまったときにも debug と言えるようにするため。

`release` と `同梱` を静かにしたのは、**配布物のパスは `target/` を含まない**から。
ここを点灯させると実ユーザーの通常運転で警告が出っぱなしになり、出っぱなしの警告は読まれなくなる。

判定は 1 か所（`ServerProfile`）に置き、**画面とログが同じ判定を使う**。
profile は起動時に 1 度決めて持ち回る（`OnceLock`。サーバーを起こし直しても同じ実体が選ばれる）。

**出す先は 2 か所ある。** `DawApp` は自前の描画ループを持っていて app 側の overlay を通らない。
ぶつ切りが出たのはまさに DAW 画面なので、片方だけに出しても意味がない。
描画そのものは `cmrt-tui-core` の `server_profile_badge` 1 つで、両方がそれを呼ぶ。

### 実体がソースより古いときも点灯させる

PATH 解決があったころの罠は「debug が静かに選ばれる」だった。それを潰した代わりに
**「兄弟 repo を直して debug だけ建て、TUI は古い release を掴み続ける」**が生まれる。
`release` は通常運転扱いなので素性だけでは何も言えず、「直したのに変わらない」として現れる。
穴を移しただけにしないため、これも機械で見る。

比べる相手は **その実体が置かれている `target/` を持つ cargo workspace のソース**。
その実体を建てたのがそのソースなので、他所の repo を持ち出さずに済む。

- 見るのは `*.rs` と `Cargo.toml` / `Cargo.lock` だけ。README を直しただけで点灯すると、
  点きっぱなしの警告になって読まれなくなる
- `target` と dot ディレクトリは走査から外す。外さないと成果物を数万件なめることになる
- **判定は起動時の 1 回きり**（実体の解決と同じ `OnceLock`）。毎フレーム走査はしない
- 配布物（`target/` の外）とテストの偽サーバーは判定しない。比べるソースが無いし、
  実ユーザーの PC で毎回ファイル走査をしても得るものが無い

## 結果

- 実体の決定（`resolve_server_binary`）・profile 判定（`ServerProfile`）・新しさの判定（`stale_source`）は
  `realtime-play/src/server_binary.rs` の 1 か所。`process.rs` は決まった実体を `Command` にするだけで探索しない
- ログの `action=server-resolved` に `source` / `profile` / `fullpath` が揃って出る。
  実体がソースより古いときは `stale_by_s=` と `newest_source=` も付く。どれも無ければ `action=server-not-found searched=...`
- テストは 3 経路それぞれと、**PATH に debug のサーバーを置いても選ばれない**こと、どれも無いときに
  探した場所つきで止まること、debug のパスで画面の文字列が変わること、ソースより古いときに点灯することを固定している。
  パス解決のテストは実ファイルを temp に作る（ユーザーの実環境に依存しない）

**罠:** 新しさのテストで mtime を「作った順」に頼ると、同じ tick に落ちたときだけ落ちる flaky になる。
`File::set_modified` で明示的に置く。

## この決定が残している穴

- **判定は mtime だけ。** ソースを触っただけ（内容が同じ）でも点灯する。逆に、ビルド後に
  ソースの mtime を戻すような操作をすれば黙る。ビルド ID を実体へ埋めて突き合わせるほうが
  正確だが、2 repo に版を持ち込む話になるので採らなかった
- **配布物の古さは分からない。** 判定できるのは開発ビルドの配置だけ

## 注意

- **開発中に実体を `./target/release` へ cp して 2 番で解決させようとしないこと**（`AGENTS.md` の禁止事項）。
  2 番が効くのは両方の exe が揃った形で配置されているときで、手で置くための経路ではない。
  開発機で別の実体を使いたいときは 1 番（コマンドライン引数）
- **play server をメンテしたら release をビルドする。** 3 番が見るのは release で、
  debug は先読みが 4〜5 倍遅い。デバッガを当てるなど debug を動かしたいときだけ 1 番で明示する。
  建て忘れはバッジが「ソースより古い」と言うので、気づけないままにはならない
- **兄弟 repo のパスは「この repo の親 / clap-mml-play-server」。** 既存の python スクリプトが
  同じ規則を持っている（`scripts/capture_daw_live_mix.py` の `PLAY_SERVER_ROOT`）。綴りを揃えること
- supervisor はサーバーが落ちたら起こし直す。**解決は起動時の 1 度だけ**なので、
  起こし直しでも同じ実体が選ばれる（途中で入れ替わると、画面に出した profile が嘘になる）
- 2 repo 横断のローカルビルド（`scripts/cross_repo_local.py`）とは**別の話**。あれは Rust の依存解決、これは実行するバイナリの選択
- **`offline_render_server_command`（render server 側）は今回の対象外。**
  あちらは同じ形の PATH 解決を残している。直すなら別 ADR
