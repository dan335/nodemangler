@echo off
setlocal
rem Cut a release:
rem   1. verify the working tree is clean and the tag doesn't exist
rem   2. run the full test suite
rem   3. set the workspace version in app\Cargo.toml
rem   4. commit, tag vX.Y.Z, and push
rem GitHub Actions (.github\workflows\release.yml) then builds Windows, Linux,
rem and macOS executables and publishes them to GitHub Releases.
rem
rem usage: scripts\release.bat 1.1.0

cd /d "%~dp0.."

if "%~1"=="" (
  echo usage: release.bat ^<version^>   e.g. release.bat 1.1.0
  exit /b 1
)
set VERSION=%~1

for /f "delims=" %%A in ('git status --porcelain') do goto :dirty

git rev-parse -q --verify refs/tags/v%VERSION% >nul 2>&1
if not errorlevel 1 (
  echo error: tag v%VERSION% already exists
  exit /b 1
)

echo === Running tests ===
pushd app
cargo test --workspace
if errorlevel 1 ( popd & exit /b 1 )
popd

echo === Setting version to %VERSION% ===
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\set_version.ps1 %VERSION%
if errorlevel 1 exit /b 1

pushd app
cargo check --quiet
if errorlevel 1 ( popd & exit /b 1 )
popd

git add app/Cargo.toml app/Cargo.lock || exit /b 1
git commit -m "Release v%VERSION%" || exit /b 1
git tag v%VERSION% || exit /b 1

echo === Pushing ===
git push origin HEAD v%VERSION% || exit /b 1

echo.
echo Tag v%VERSION% pushed. GitHub Actions is now building Windows, Linux, and
echo macOS executables and will publish them to GitHub Releases.
echo Watch progress under the repository's Actions tab.
exit /b 0

:dirty
echo error: working tree is not clean - commit or stash first
exit /b 1
