# Compatibility wrapper. The Rust helper owns templates, validation and builds.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Name,
    [ValidateSet('callbacks','graphics')][string]$Kind='callbacks'
)
$ErrorActionPreference='Stop'
$projectRoot=(Get-Item -LiteralPath (Join-Path $PSScriptRoot '..')).FullName
cargo run --quiet --manifest-path (Join-Path $projectRoot 'Cargo.toml') --package cargo-rekenshi -- new plugin $Name --kind $Kind --path (Join-Path $projectRoot "mods-src\$Name")
if ($LASTEXITCODE -ne 0) { throw 'Plugin creation failed; see helper output above.' }
