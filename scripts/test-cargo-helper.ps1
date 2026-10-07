[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$projectRoot=(Get-Item -LiteralPath (Join-Path $PSScriptRoot '..')).FullName
$helper=Join-Path $projectRoot 'target\release\cargo-rekenshi.exe'
$sdk=Join-Path $projectRoot 'crates\plugin-sdk'
$checksRoot=Join-Path $projectRoot ('target\cargo-helper-tests\'+[guid]::NewGuid().ToString('N'))
[System.IO.Directory]::CreateDirectory($checksRoot) | Out-Null
foreach ($kind in @('callbacks','graphics')) {
    $folder=Join-Path $checksRoot "$kind mod"
    $log=Join-Path $checksRoot "$kind.log"
    & $helper rekenshi new plugin "helper-$kind" --kind $kind --path $folder --sdk-path $sdk > $log 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Starter failed; read $log" }
    $dll=Join-Path $folder ("target\release\helper_$kind.dll")
    if (-not (Test-Path -LiteralPath $dll)) { throw "Missing generated DLL: $dll" }
    $source=Join-Path $folder 'src\implementation.rs'
    [System.IO.File]::WriteAllText($source,'user-authored implementation')
    $before=(Get-FileHash -LiteralPath $source).Hash
    & $helper new plugin "helper-$kind" --path $folder > (Join-Path $checksRoot "$kind-repeat.log") 2>&1
    if ($LASTEXITCODE -eq 0) { throw 'Existing project was accepted.' }
    if ((Get-FileHash -LiteralPath $source).Hash -ne $before) { throw 'Existing source was overwritten.' }
}
Write-Output 'PASS: Cargo helper callback/graphics DLL builds, paths with spaces, external Cargo argument convention, and authored-file preservation.'
