@echo off
cd /d "%~dp0"
node "scripts\native-dm.mjs" %*
if errorlevel 1 pause
