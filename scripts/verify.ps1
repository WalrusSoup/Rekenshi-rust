[CmdletBinding()]
param([string]$KenshiDir)

$ErrorActionPreference = 'Stop'
$projectRoot = (Get-Item -LiteralPath (Join-Path $PSScriptRoot '..')).FullName
Push-Location -LiteralPath $projectRoot
try {
    python (Join-Path $PSScriptRoot 'test-symbols.py')
    if ($LASTEXITCODE -ne 0) { throw 'Native symbol scanner tests failed.' }
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Formatting check failed.' }
    cargo clippy --all-targets --workspace --locked -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Clippy failed.' }
    cargo test --all-targets --workspace --locked
    if ($LASTEXITCODE -ne 0) { throw 'Workspace tests failed.' }
    cargo test --doc --workspace --locked
    if ($LASTEXITCODE -ne 0) { throw 'Documentation examples failed.' }
    $oldRustdocFlags = $env:RUSTDOCFLAGS
    try {
        $env:RUSTDOCFLAGS = "$oldRustdocFlags -D warnings".Trim()
        cargo doc -p rekenshi-plugin-api -p rekenshi-plugin-runtime -p rekenshi-plugin-sdk -p rekenshi-weather -p rekenshi-dark-ui --no-deps --locked
        if ($LASTEXITCODE -ne 0) { throw 'Public interface documentation failed.' }
    } finally { $env:RUSTDOCFLAGS = $oldRustdocFlags }
    cargo build --release --workspace --locked
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    & (Join-Path $PSScriptRoot 'test-cargo-helper.ps1')
    & (Join-Path $PSScriptRoot 'test-install.ps1')
    & '.\target\release\plugin-smoke.exe' '.\target\release\RE_Kenshi_Rust.dll'
    if ($LASTEXITCODE -ne 0) { throw 'DLL bootstrap smoke check failed.' }
    if ($KenshiDir) {
        & '.\target\release\hook-list.exe' '.\target\release\RE_Kenshi_Rust.dll' $KenshiDir
        if ($LASTEXITCODE -ne 0) { throw 'Named hook inventory failed.' }
        & '.\target\release\plugin-smoke.exe' '.\target\release\RE_Kenshi_Rust.dll' $KenshiDir
        if ($LASTEXITCODE -ne 0) { throw 'Ogre detour smoke check failed.' }
        & (Join-Path $PSScriptRoot 'test-plugins.ps1') -KenshiDir $KenshiDir
        python (Join-Path $PSScriptRoot 'test-enhancements.py') $KenshiDir
        if ($LASTEXITCODE -ne 0) { throw 'Native enhancement integration tests failed.' }
        python (Join-Path $PSScriptRoot 'test-modpack.py') $KenshiDir
        if ($LASTEXITCODE -ne 0) { throw 'Built-in mod pack integration tests failed.' }
        cmd /c (Join-Path $PSScriptRoot 'build-menu-fixture.cmd')
        if ($LASTEXITCODE -ne 0) { throw 'Native menu fixture build failed.' }
        python (Join-Path $PSScriptRoot 'test-menu.py') $KenshiDir
        if ($LASTEXITCODE -ne 0) { throw 'Native Options tab lifecycle tests failed.' }
        python (Join-Path $PSScriptRoot 'test-dark-ui.py') $KenshiDir
        if ($LASTEXITCODE -ne 0) { throw 'Native whole-game dark UI colour/lifetime tests failed.' }
        python (Join-Path $PSScriptRoot 'test-sdk.py') $KenshiDir
        if ($LASTEXITCODE -ne 0) { throw 'Independent Rust SDK plugin integration failed.' }
        python (Join-Path $PSScriptRoot 'test-gameplay.py') $KenshiDir
        if ($LASTEXITCODE -ne 0) { throw 'Independent gameplay SDK integration failed.' }
        python (Join-Path $PSScriptRoot 'test-weather.py') $KenshiDir
        if ($LASTEXITCODE -ne 0) { throw 'Native rain render integration tests failed.' }
        cmd /c (Join-Path $PSScriptRoot 'build-native-fixture.cmd')
        if ($LASTEXITCODE -ne 0) { throw 'Native exception/reference fixture build failed.' }
        python (Join-Path $PSScriptRoot 'test-performance.py') $KenshiDir
        if ($LASTEXITCODE -ne 0) { throw 'Native DDS, SIC or collider integration tests failed.' }
        $oldTestGame = $env:REKENSHI_TEST_GAME
        try {
            $env:REKENSHI_TEST_GAME = $KenshiDir
            cargo test -p rekenshi-rust --lib --locked managed_callbacks_forward_native_ogre_and_mygui_and_stop_cleanly -- --ignored
            if ($LASTEXITCODE -ne 0) { throw 'Managed Ogre/MyGUI callback integration test failed.' }
            cargo test -p rekenshi-graphics -p rekenshi-heightmap-cache --lib --locked -- --ignored
            if ($LASTEXITCODE -ne 0) { throw 'GPU output/state or native heightmap differential test failed.' }
        } finally { $env:REKENSHI_TEST_GAME = $oldTestGame }
    }
} finally {
    Pop-Location
}
