[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$projectRoot = (Get-Item -LiteralPath (Join-Path $PSScriptRoot '..')).FullName
$gameRoot = Join-Path $projectRoot ('target\install-tests\' + [guid]::NewGuid().ToString('N'))
[System.IO.Directory]::CreateDirectory($gameRoot) | Out-Null
[System.IO.File]::WriteAllBytes((Join-Path $gameRoot 'OgreMain_x64.dll'), [byte[]]::new(0))
[System.IO.File]::WriteAllBytes((Join-Path $gameRoot 'kenshi_x64.exe'), [byte[]]::new(0))
$config = Join-Path $gameRoot 'Plugins_x64.cfg'
[System.IO.File]::WriteAllText($config, "PluginFolder=.\`r`nPlugin=RE_Kenshi`r`nPlugin=RenderSystem_Direct3D11_x64`r`n")
$originalHash = (Get-FileHash -LiteralPath $config).Hash
$installer = Join-Path $PSScriptRoot 'install.ps1'
& $installer -KenshiDir $gameRoot -WhatIf | Out-Host
if (Test-Path -LiteralPath (Join-Path $gameRoot 'RE_Kenshi_Rust.dll')) { throw 'WhatIf installed files.' }
if ((Get-FileHash -LiteralPath $config).Hash -ne $originalHash) { throw 'WhatIf changed config.' }
$legacyModRoot = Join-Path $gameRoot 'RE_Kenshi_Rust'
[System.IO.Directory]::CreateDirectory($legacyModRoot) | Out-Null
[System.IO.File]::WriteAllText((Join-Path $legacyModRoot 'performance.toml'), "shader_cache=false`nmax_cache_mb=128`n")
[System.IO.File]::WriteAllText((Join-Path $legacyModRoot 'plugins.toml'), "schema_version=1`n[[plugins]]`nfile='rekenshi_performance.dll'`nenabled=true`n[[plugins]]`nfile='rekenshi_diagnostics.dll'`nenabled=true`n")
& $installer -KenshiDir $gameRoot | Out-Host
$modRoot = Join-Path $gameRoot 'RE_Kenshi_Rust'
if (-not (Test-Path -LiteralPath (Join-Path $modRoot 'texture-pack.exe'))) { throw 'Texture converter missing.' }
foreach ($legacyDll in @('rekenshi_shader_cache.dll', 'rekenshi_graphics.dll', 'rekenshi_color_grade.dll', 'rekenshi_diagnostics.dll')) {
    if (Test-Path -LiteralPath (Join-Path $modRoot "plugins\$legacyDll")) { throw "Installer shipped a separate plugin DLL: $legacyDll" }
}
if (-not [System.IO.File]::ReadAllText((Join-Path $modRoot 'shader-cache.toml')).Contains('max_cache_mb=128')) { throw 'Shader rename lost legacy cache budget.' }
if (-not [System.IO.File]::ReadAllText((Join-Path $modRoot 'modpack.toml')).Contains('"performance.shader_cache" = false')) { throw 'Shader rename lost disabled choice.' }
if ([System.IO.File]::ReadAllText((Join-Path $modRoot 'plugins.toml')).Contains('rekenshi_performance.dll')) { throw 'Old performance entry was not migrated.' }
if (-not [System.IO.File]::ReadAllText((Join-Path $modRoot 'plugins.toml')).Contains('discover = true')) { throw 'Upgrade did not enable folder discovery.' }
$modConfig = Join-Path $modRoot 'plugins.toml'
[System.IO.File]::WriteAllText($modConfig, "schema_version=1`ndiscover=false`n[[plugins]]`nfile='rekenshi_diagnostics.dll'`nenabled=false`n")
$userHash = (Get-FileHash -LiteralPath $modConfig).Hash
$packConfig = Join-Path $modRoot 'modpack.toml'
$oldPack = "schema_version=1`n[settings.values]`n'performance.shader_cache'=false`n'graphics.color_grade'=true`n'graphics.contrast'=1.2`n'weather.rain_refinement'=true`n"
[System.IO.File]::WriteAllText($packConfig, $oldPack)
$samplesBackup = Join-Path $modRoot 'modpack.toml.before-dll-plugins'
# Fresh legacy migration may have created its own snapshot. Use a separate
# synthetic upgrade folder state to assert this snapshot contains our old pack.
if (Test-Path -LiteralPath $samplesBackup) { Remove-Item -LiteralPath $samplesBackup }
$custom = Join-Path $modRoot 'plugins\custom.dll'
[System.IO.File]::WriteAllText($custom, 'user-owned-plugin')
& $installer -KenshiDir $gameRoot | Out-Host
$migratedPack = [System.IO.File]::ReadAllText($packConfig)
if ($migratedPack.Contains('graphics.color_grade') -or $migratedPack.Contains('weather.rain_refinement')) { throw 'Retired visual settings remain active.' }
if (-not $migratedPack.Contains('"performance.shader_cache" = false')) { throw 'Sample migration lost performance choice.' }
if ([System.IO.File]::ReadAllText($samplesBackup) -ne $oldPack) { throw 'Sample migration lost original settings.' }
$migratedHash = (Get-FileHash -LiteralPath $packConfig).Hash
& $installer -KenshiDir $gameRoot | Out-Host
if ((Get-FileHash -LiteralPath $packConfig).Hash -ne $migratedHash) { throw 'Sample migration is not idempotent.' }
if ((Get-FileHash -LiteralPath $modConfig).Hash -ne $userHash) { throw 'Update overwrote plugin choices.' }
if ([System.IO.File]::ReadAllText($custom) -ne 'user-owned-plugin') { throw 'Update changed another plugin DLL.' }
# Explicit sample installation is separate from the core and retains settings.
& $installer -KenshiDir $gameRoot -IncludeKenshi2Sample | Out-Host
if ((Get-FileHash -LiteralPath (Join-Path $modRoot 'plugins\rekenshi_kenshi2_style.dll')).Hash -ne
    (Get-FileHash -LiteralPath (Join-Path $projectRoot 'target\release\rekenshi_kenshi2_style.dll')).Hash) { throw 'Sample DLL was not installed.' }
if ((Get-FileHash -LiteralPath $modConfig).Hash -ne $userHash) { throw 'Sample install overwrote explicit discovery choice.' }
if (-not [System.IO.File]::ReadAllText($config).Contains('Plugin=RE_Kenshi_Rust')) { throw 'Bootstrap plugin missing.' }
& $installer -KenshiDir $gameRoot -Action Restore | Out-Host
if ((Get-FileHash -LiteralPath $config).Hash -ne $originalHash) { throw 'Restore changed original bytes.' }
Write-Output 'PASS: core and optional sample install, WhatIf, settings migration, explicit discovery/choices/custom DLL preservation, and exact config restore.'
