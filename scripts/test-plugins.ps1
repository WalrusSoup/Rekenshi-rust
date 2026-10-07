[CmdletBinding()]
param([Parameter(Mandatory)][string]$KenshiDir)
$ErrorActionPreference = 'Stop'
$projectRoot = (Get-Item -LiteralPath (Join-Path $PSScriptRoot '..')).FullName
$buildRoot = Join-Path $projectRoot 'target\release'
$caseRoot = Join-Path $projectRoot ('target\plugin-tests\' + [guid]::NewGuid().ToString('N'))
$hostDll = Join-Path $buildRoot 'RE_Kenshi_Rust.dll'
$smoke = Join-Path $buildRoot 'plugin-smoke.exe'

function Test-Case([string]$Name, [string]$Config, [int]$Active, [int]$Failed) {
    $caseDir = Join-Path $caseRoot $Name
    $modRoot = Join-Path $caseDir 'RE_Kenshi_Rust'
    $modDir = Join-Path $modRoot 'plugins'
    [System.IO.Directory]::CreateDirectory($modDir) | Out-Null
    Copy-Item -LiteralPath $hostDll -Destination $caseDir
    foreach ($file in @('rekenshi_diagnostics.dll', 'fixture_incompatible.dll', 'fixture_reject.dll', 'fixture_lifecycle.dll')) {
        Copy-Item -LiteralPath (Join-Path $buildRoot $file) -Destination $modDir
    }
    Copy-Item -LiteralPath $hostDll -Destination (Join-Path $modDir 'no_entry.dll')
    Copy-Item -LiteralPath (Join-Path $buildRoot 'rekenshi_diagnostics.dll') -Destination (Join-Path $modDir 'duplicate_id.dll')
    [System.IO.File]::WriteAllText((Join-Path $modRoot 'plugins.toml'), $Config)
    & $smoke (Join-Path $caseDir 'RE_Kenshi_Rust.dll') $KenshiDir $Active $Failed | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Plugin integration case failed: $Name" }
    Write-Host "PASS plugin case: $Name"
    return (Get-Content -LiteralPath (Join-Path $caseDir 'RE_Kenshi_Rust.log') -Raw)
}

$valid = @'
schema_version = 1
[[plugins]]
file = "rekenshi_diagnostics.dll"
enabled = true
'@
$log = Test-Case 'enabled' $valid 1 0
if (-not $log.Contains('[diagnostics] started #1;') -or -not $log.Contains('[diagnostics] started #2;') -or $log.Contains('[diagnostics] started #3;')) {
    throw 'Plugin startup/restart/idempotence trace mismatch.'
}

$null = Test-Case 'disabled-missing' @'
schema_version = 1
[[plugins]]
file = "absent.dll"
enabled = false
'@ 0 0

foreach ($badFile in @('fixture_incompatible.dll', 'fixture_reject.dll', 'no_entry.dll', 'absent.dll', 'duplicate_id.dll')) {
    $caseConfig = $valid + "`n[[plugins]]`nfile = `"$badFile`"`nenabled = true`n"
    $log = Test-Case $badFile.Replace('.dll', '') $caseConfig 1 1
    if ($badFile -eq 'fixture_reject.dll' -and -not $log.Contains('[fixture-reject] failed start cleaned up')) {
        throw 'Failed startup did not invoke cleanup.'
    }
}

$null = Test-Case 'path-escape' @'
schema_version = 1
[[plugins]]
file = "../evil.dll"
enabled = true
'@ 0 1
$null = Test-Case 'unsupported-schema' 'schema_version = 99' 0 1
$null = Test-Case 'malformed-config' 'schema_version = [broken' 0 1
$null = Test-Case 'duplicate-filename' ($valid + "`n[[plugins]]`nfile = `"REKENSHI_DIAGNOSTICS.DLL`"`nenabled = true") 0 1

$ordered = $valid + "`n[[plugins]]`nfile = `"fixture_lifecycle.dll`"`nenabled = true"
$log = Test-Case 'lifecycle-order' $ordered 2 0
$firstStart = $log.IndexOf('[diagnostics] started #1;')
$secondStart = $log.IndexOf('[fixture-lifecycle] start')
$secondStop = $log.IndexOf('[fixture-lifecycle] stop')
$firstStop = $log.IndexOf('[diagnostics] stopped;')
if ($firstStart -lt 0 -or $secondStart -le $firstStart -or $secondStop -le $secondStart -or $firstStop -le $secondStop) {
    throw 'Plugins did not start in config order and stop in reverse order.'
}
Write-Output "PASS: plugin integration suite; retained logs in $caseRoot"
