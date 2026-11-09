@echo off
cd /d "%~dp0"
node "scripts\build-native.mjs"
if errorlevel 1 (
  echo Workspace update failed. The previous app selection was kept.
  pause
  exit /b 1
)
echo Workspace update complete. Open Start-Dungeon-World-AI.cmd for live testing.
pause
