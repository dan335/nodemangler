# Sets the workspace version in app/Cargo.toml. Used by release.bat;
# release.sh does the same thing with perl.
param([Parameter(Mandatory = $true)][string]$Version)

if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    Write-Error "version must be X.Y.Z, got '$Version'"
    exit 1
}

$path = Join-Path $PSScriptRoot "..\app\Cargo.toml"
$content = Get-Content $path -Raw
$updated = $content -replace '(?s)(\[workspace\.package\].*?version = ")[^"]+(")', "`${1}$Version`${2}"

if ($updated -eq $content) {
    Write-Error "could not find the [workspace.package] version line in app/Cargo.toml"
    exit 1
}

Set-Content -Path $path -Value $updated -NoNewline
Write-Host "app/Cargo.toml version set to $Version"
