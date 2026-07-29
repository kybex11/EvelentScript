@echo off
setlocal EnableExtensions

cd /d "%~dp0"
set ROOT=%~dp0..\..
set EXT=%CD%

echo.
echo === Extension only (needs lib/evelentscript) ===
if not exist "%ROOT%\lib\evelentscript\index.js" (
  echo lib/evelentscript not found. Run build.bat from repo root first.
  exit /b 1
)

if not exist node_modules call npm install
if errorlevel 1 exit /b 1

call npx --yes @vscode/vsce package
if errorlevel 1 exit /b 1

echo.
echo [OK] %EXT%\evelentscript-*.vsix
endlocal
exit /b 0
