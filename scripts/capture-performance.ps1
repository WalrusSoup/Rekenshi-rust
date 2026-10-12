<#
.SYNOPSIS
Records a bounded Kenshi performance trace and process memory samples.
.EXAMPLE
.\scripts\capture-performance.ps1 -Scenario town -Seconds 60 -Notes 'Save name; camera; speed; mods'
#>
[CmdletBinding(SupportsShouldProcess)]
param(
    [ValidateSet('town', 'battle', 'travel')]
    [string]$Scenario = 'town',
    [ValidateRange(10, 300)]
    [int]$Seconds = 60,
    [int]$GameProcessId = 0,
    [string]$Notes = ''
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Get-Item -LiteralPath (Join-Path $PSScriptRoot '..')).FullName
$captureId = (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N').Substring(0, 8)
$outputDir = Join-Path $projectRoot "target\performance\$Scenario-$captureId"
$instance = "Kenshi-$captureId"
$wpr = (Get-Command wpr.exe -ErrorAction Stop).Source
$profiles = @('CPU', 'GPU')
if ($Scenario -eq 'travel') { $profiles += @('DiskIO', 'FileIO') }
$startArgs = @()
foreach ($profile in $profiles) { $startArgs += @('-start', $profile) }
$startArgs += @('-filemode', '-instancename', $instance)

if (-not $PSCmdlet.ShouldProcess($outputDir, "Record $Seconds seconds using WPR profiles $($profiles -join ', ') and process memory samples")) { return }
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run this script in an Administrator PowerShell window; WPR needs elevated tracing access.'
}
if ($GameProcessId -ne 0) {
    $game = Get-Process -Id $GameProcessId
    if ($game.ProcessName -notlike '*kenshi*') { throw 'The selected process is not named Kenshi.' }
} else {
    $games = @(Get-Process -Name '*kenshi*' -ErrorAction SilentlyContinue)
    if ($games.Count -ne 1) { throw 'Load a Kenshi save first. If multiple Kenshi processes exist, pass -GameProcessId.' }
    $game = $games[0]
}

[System.IO.Directory]::CreateDirectory($outputDir) | Out-Null
$tracePath = Join-Path $outputDir 'trace.etl'
$modules = @()
$moduleInventoryError = $null
try {
    $modules = @($game.Modules | ForEach-Object {
        [pscustomobject]@{ Name = $_.ModuleName; Path = $_.FileName; BaseAddress = ('0x{0:X}' -f $_.BaseAddress.ToInt64()); Size = $_.ModuleMemorySize }
    })
} catch { $moduleInventoryError = $_.Exception.Message }
$metadata = [ordered]@{
    Scenario = $Scenario; RequestedSeconds = $Seconds; Notes = $Notes
    ProcessId = $game.Id; Executable = $game.Path
    ExecutableSha256 = (Get-FileHash -LiteralPath $game.Path -Algorithm SHA256).Hash
    ProcessStartUtc = $game.StartTime.ToUniversalTime().ToString('o')
    Profiles = $profiles; WprInstance = $instance; Modules = $modules
    ModuleInventoryError = $moduleInventoryError
    CaptureStartUtc = $null; CaptureEndUtc = $null; TraceSaved = $false
}
$metadataPath = Join-Path $outputDir 'capture.json'
$metadata | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $metadataPath -Encoding UTF8
$samples = [System.Collections.Generic.List[object]]::new()
$started = $false
try {
    & $wpr @startArgs
    if ($LASTEXITCODE -ne 0) { throw 'WPR could not start. Existing recordings were not cancelled.' }
    $started = $true
    $metadata.CaptureStartUtc = [DateTime]::UtcNow.ToString('o')
    Write-Host "Recording $Scenario for $Seconds seconds. Switch back to Kenshi and reproduce the scenario."
    Write-Host 'WPR records system-wide activity; keep the trace local. Analysis will filter to the Kenshi process.'
    $timer = [Diagnostics.Stopwatch]::StartNew()
    while ($timer.Elapsed.TotalSeconds -lt $Seconds) {
        $game.Refresh()
        if ($game.HasExited) { Write-Warning 'Kenshi exited; saving the partial capture.'; break }
        $samples.Add([pscustomobject]@{
            Utc = [DateTime]::UtcNow.ToString('o')
            ElapsedSeconds = $timer.Elapsed.TotalSeconds
            ProcessId = $game.Id
            CpuSeconds = $game.TotalProcessorTime.TotalSeconds
            PrivateBytes = $game.PrivateMemorySize64
            WorkingSetBytes = $game.WorkingSet64
            Threads = $game.Threads.Count
        })
        Start-Sleep -Milliseconds 1000
    }
} finally {
    if ($started) {
        $metadata.CaptureEndUtc = [DateTime]::UtcNow.ToString('o')
        & $wpr -stop $tracePath "Kenshi $Scenario" -instancename $instance
        $metadata.TraceSaved = ($LASTEXITCODE -eq 0)
        if (-not $metadata.TraceSaved) {
            Write-Warning "Trace save failed. Recover this instance with: wpr -stop `"$tracePath`" -instancename $instance"
        }
        $samples | Export-Csv -LiteralPath (Join-Path $outputDir 'memory.csv') -NoTypeInformation
    }
    $metadata | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $metadataPath -Encoding UTF8
}
if (-not $metadata.TraceSaved) { throw "No completed trace. Inspect $outputDir" }
Write-Host "Capture saved: $outputDir"
