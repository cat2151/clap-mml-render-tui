@echo off
chcp 65001 > nul
rem いまの状態を表示する。commit して安全でなければ非 0 で終了する。
rem 実体は scripts\cross_repo_local.py。この .bat は Windows 用の入口。
where /q python
if errorlevel 1 (
    py -3 "%~dp0scripts\cross_repo_local.py" status %*
) else (
    python "%~dp0scripts\cross_repo_local.py" status %*
)
