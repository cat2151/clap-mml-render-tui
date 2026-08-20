@echo off
chcp 65001 > nul
rem ローカル横断ビルドを解除し、Cargo.lock を push 済みの最新 HEAD へ張り直す。
rem 実体は scripts\cross_repo_local.py。この .bat は Windows 用の入口。
where /q python
if errorlevel 1 (
    py -3 "%~dp0scripts\cross_repo_local.py" off %*
) else (
    python "%~dp0scripts\cross_repo_local.py" off %*
)
