[CmdletBinding(SupportsShouldProcess)]
param(
    [Parameter(Mandatory)][string]$KenshiDir,
    [ValidateSet('Install', 'Restore')][string]$Action = 'Install',
    [string]$DllPath = (Join-Path $PSScriptRoot '..\target\release\RE_Kenshi_Rust.dll'),
    [string]$PluginConfigPath = (Join-Path $PSScriptRoot '..\config\plugins.toml'),
    [string]$EnhancementsBuildDir = (Join-Path $PSScriptRoot '..\target\release'),
    [switch]$IncludeKenshi2Sample
)

$ErrorActionPreference = 'Stop'
$gameRoot = (Get-Item -LiteralPath $KenshiDir).FullName
if (-not (Test-Path -LiteralPath (Join-Path $gameRoot 'OgreMain_x64.dll'))) {
    throw 'This directory does not contain OgreMain_x64.dll.'
}
if (-not ((Test-Path -LiteralPath (Join-Path $gameRoot 'kenshi_x64.exe')) -or
          (Test-Path -LiteralPath (Join-Path $gameRoot 'kenshi_GOG_x64.exe')))) {
    throw 'This directory does not contain the Kenshi executable.'
}
if (Get-Process -Name 'kenshi_x64', 'kenshi_GOG_x64' -ErrorAction SilentlyContinue) {
    throw 'Close Kenshi before installing or restoring this plugin.'
}

$configPath = Join-Path $gameRoot 'Plugins_x64.cfg'
$backupDir = Join-Path $gameRoot 'RE_Kenshi_Rust.bootstrap-backup'
$originalPath = Join-Path $backupDir 'Plugins_x64.cfg.original'
$installedPath = Join-Path $backupDir 'Plugins_x64.cfg.installed'
$destination = Join-Path $gameRoot 'RE_Kenshi_Rust.dll'

if ($Action -eq 'Restore') {
    if (-not (Test-Path -LiteralPath $originalPath) -or -not (Test-Path -LiteralPath $installedPath)) {
        throw 'No complete bootstrap configuration backup exists.'
    }
    $currentHash = (Get-FileHash -LiteralPath $configPath).Hash
    $originalHash = (Get-FileHash -LiteralPath $originalPath).Hash
    $installedHash = (Get-FileHash -LiteralPath $installedPath).Hash
    if ($currentHash -eq $originalHash) {
        Write-Output 'Original configuration is already restored.'
        return
    }
    if ($currentHash -ne $installedHash) {
        throw "Plugin configuration changed after installation. Merge the saved original manually: $originalPath"
    }
    if ($PSCmdlet.ShouldProcess($configPath, 'Restore saved plugin configuration')) {
        Copy-Item -LiteralPath $originalPath -Destination $configPath -Force
        Write-Output "Restored $configPath. The dormant Rust DLL and backup are retained."
    }
    return
}

$source = (Get-Item -LiteralPath $DllPath).FullName
$textureConverterSource = (Get-Item -LiteralPath (Join-Path $EnhancementsBuildDir 'texture-pack.exe')).FullName
$pluginConfigSource = (Get-Item -LiteralPath $PluginConfigPath).FullName
$modRoot = Join-Path $gameRoot 'RE_Kenshi_Rust'
$modDir = Join-Path $modRoot 'plugins'
$modConfig = Join-Path $modRoot 'plugins.toml'
if ($IncludeKenshi2Sample) { $sampleSource=(Get-Item -LiteralPath (Join-Path $EnhancementsBuildDir 'rekenshi_kenshi2_style.dll')).FullName }
if ($source -eq $destination) {
    throw 'Build output and install destination must be different files.'
}
if (-not (Test-Path -LiteralPath $configPath)) {
    throw 'Plugins_x64.cfg is missing.'
}
$currentHash = (Get-FileHash -LiteralPath $configPath).Hash
$hasBackup = Test-Path -LiteralPath $originalPath
if ($hasBackup) {
    if (-not (Test-Path -LiteralPath $installedPath)) {
        throw 'Incomplete backup; inspect the backup directory before proceeding.'
    }
    $originalHash = (Get-FileHash -LiteralPath $originalPath).Hash
    $installedHash = (Get-FileHash -LiteralPath $installedPath).Hash
    if ($currentHash -ne $originalHash -and $currentHash -ne $installedHash) {
        throw 'Plugin configuration changed since the previous install. Resolve it before updating.'
    }
} elseif (Test-Path -LiteralPath $destination) {
    throw 'An unmanaged RE_Kenshi_Rust.dll already exists. Preserve it before installing.'
}

$basePath = if ($hasBackup) { $originalPath } else { $configPath }
$reader = [System.IO.StreamReader]::new($basePath, $true)
try {
    $original = $reader.ReadToEnd()
    $encoding = $reader.CurrentEncoding
} finally {
    $reader.Dispose()
}
$newline = if ($original.Contains("`r`n")) { "`r`n" } else { "`n" }
$lines = [System.Collections.Generic.List[string]]::new()
$inserted = $false
foreach ($line in [regex]::Split($original, '\r?\n')) {
    # Replace the original RE_Kenshi entry; retain all other engine plugins.
    if ($line -match '^\s*Plugin\s*=\s*(RE_Kenshi|RE_Kenshi_Rust)\s*$') { continue }
    if (-not $inserted -and $line -match '^\s*Plugin\s*=') {
        $lines.Add('Plugin=RE_Kenshi_Rust')
        $inserted = $true
    }
    $lines.Add($line)
}
if (-not $inserted) { throw 'No engine plugin entries found in Plugins_x64.cfg.' }
$replacement = [string]::Join($newline, $lines)

if ($PSCmdlet.ShouldProcess($gameRoot, 'Back up plugin configuration and install Rust bootstrap')) {
    [System.IO.Directory]::CreateDirectory($backupDir) | Out-Null
    if (-not $hasBackup) {
        Copy-Item -LiteralPath $configPath -Destination $originalPath
    }
    Copy-Item -LiteralPath $source -Destination $destination -Force
    [System.IO.Directory]::CreateDirectory($modDir) | Out-Null
    if ($IncludeKenshi2Sample) { Copy-Item -LiteralPath $sampleSource -Destination (Join-Path $modDir 'rekenshi_kenshi2_style.dll') -Force }
    Copy-Item -LiteralPath $textureConverterSource -Destination (Join-Path $modRoot 'texture-pack.exe') -Force
    foreach ($file in @('shader-cache.toml')) {
        $settingsDestination = Join-Path $modRoot $file
        if (-not (Test-Path -LiteralPath $settingsDestination)) {
            $legacySettings = Join-Path $modRoot 'performance.toml'
            if ($file -eq 'shader-cache.toml' -and (Test-Path -LiteralPath $legacySettings)) {
                Copy-Item -LiteralPath $legacySettings -Destination $settingsDestination
            } else {
                Copy-Item -LiteralPath (Join-Path $PSScriptRoot "..\config\$file") -Destination $settingsDestination
            }
        }
    }
    # User plugin choices survive bootstrap updates; write defaults only once.
    if (-not (Test-Path -LiteralPath $modConfig)) {
        Copy-Item -LiteralPath $pluginConfigSource -Destination $modConfig
    }
    & (Join-Path $EnhancementsBuildDir 'pack-config.exe') $modRoot
    if ($LASTEXITCODE -ne 0) { throw 'Mod pack settings migration failed.' }
    # Save the intended result before writing the live configuration. Original
    # config remains available if writing is interrupted.
    [System.IO.File]::WriteAllText($installedPath, $replacement, $encoding)
    Copy-Item -LiteralPath $installedPath -Destination $configPath -Force
    Write-Output "Installed $destination"
    Write-Output "Original plugin configuration saved in $originalPath"
    Write-Output "Mod configuration: $modConfig (existing choices preserved)"
}
