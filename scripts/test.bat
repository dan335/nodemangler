@echo off
rem Run the full test suite. Extra args are passed to cargo test.
pushd "%~dp0..\app"
cargo test --workspace %*
set EXITCODE=%ERRORLEVEL%
popd
exit /b %EXITCODE%
