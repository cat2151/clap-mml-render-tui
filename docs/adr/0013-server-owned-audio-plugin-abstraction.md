# 0013: オーディオプラグインの具象知識をserver側へ置く

## Status

Accepted

## Context

TUIがplugin ID、ファイル名、preset拡張子、カテゴリ体系、mono/poly取得方法を見て
分岐すると、対応pluginを追加するたびに画面・キャッシュ・レンダー選択を横断して
変更する必要がある。さらに、TUIとplay serverでpatchの送り先判定がずれると、別pluginが
理解できないstateを黙って無視し、操作だけ成功したように見える。

## Decision

plugin固有の判定とmetadata生成は、play-server repositoryの共有crate
`cmrt-core` / `cmrt-server-config`が所有する。TUIは次の抽象型だけを利用する。

- `PluginKey`: プロセス内・派生cache内でpluginを一意に参照するkey
- `PatchRef`: `PluginKey`と従来の表示patch文字列の組
- `AudioPluginInfo`: patch routingとvoicing方針を提供するplugin情報
- `AudioPatch`: 正規化表示名、sort/category metadata、voicing hintを含むpatch情報

保存済みMML、history、session、server wire protocolのpatch表現は互換性のため文字列のままに
する。再生成可能なpatch catalog cacheだけは`PluginKey`を含む`AudioPatch`を保存する。

patch文字列から候補を一意に決められない場合、TUIは先頭または既定pluginへfallbackせず
エラーにする。音色無指定だけは従来どおり既定pluginを使う。

## Consequences

新しいplugin adapterの追加ではserver側のprofile、patch form、metadata/voicing方針を追加する。
TUI側にplugin名を使う分岐を追加しない。既存cacheはformat version不一致として再構築する。

