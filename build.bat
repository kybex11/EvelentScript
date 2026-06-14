@echo off
setlocal EnableExtensions EnableDelayedExpansion

cd /d "%~dp0"
set ROOT=%CD%
set EXT=%ROOT%\extensions\vscode-evelentscript

echo.
echo === EvelentScript: compiler ===
call npm run build
if errorlevel 1 (
  echo.
  echo [FAIL] npm run build
  exit /b 1
)

echo.
echo === EvelentScript: extension deps ===
pushd "%EXT%"
if not exist node_modules (
  call npm install
  if errorlevel 1 (
    popd
    echo [FAIL] npm install in extension
    exit /b 1
  )
)

echo.
echo === EvelentScript: VSIX ===
call npx --yes @vscode/vsce package
set VSCE_ERR=!errorlevel!
popd

if !VSCE_ERR! neq 0 (
  echo.
  echo [FAIL] vsce package
  exit /b 1
)

echo.
echo [OK] Compiler: %ROOT%\lib\evelentscript
echo [OK] VSIX:     %EXT%\evelentscript-*.vsix
echo.
endlocal
exit /b 0
