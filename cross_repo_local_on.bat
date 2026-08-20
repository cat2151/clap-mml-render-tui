@echo off
chcp 65001 > nul
rem 兄弟 repo (..\clap-mml-play-server) の作業ツリーを直接参照するビルドへ切り替える。
rem 実体は scripts\cross_repo_local.py。この .bat は Windows 用の入口。
where /q python
if errorlevel 1 (
    py -3 "%~dp0scripts\cross_repo_local.py" on %*
) else (
    python "%~dp0scripts\cross_repo_local.py" on %*
)
