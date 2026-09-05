@echo off
chcp 65001 > nul
rem pre-commit hook を有効化する（core.hooksPath を .githooks へ向ける）。clone ごとに一度だけ。
rem 実体は scripts\cross_repo_local.py。この .bat は Windows 用の入口。
where /q python
if errorlevel 1 (
    py -3 "%~dp0scripts\cross_repo_local.py" hooks %*
) else (
    python "%~dp0scripts\cross_repo_local.py" hooks %*
)
