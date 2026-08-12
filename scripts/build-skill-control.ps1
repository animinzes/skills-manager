[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$workspaceRoot = Split-Path -Parent $repositoryRoot
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'

if (-not (Test-Path -LiteralPath $vswhere)) {
    throw 'Visual Studio Build Tools are not installed.'
}

$vsInstallPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vsInstallPath) {
    throw 'The MSVC C++ workload is not installed.'
}

$vsDevCmd = Join-Path $vsInstallPath 'Common7\Tools\VsDevCmd.bat'
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
$cargoHome = Join-Path $workspaceRoot '.cargo-home'
$cargoTarget = Join-Path $workspaceRoot '.cargo-target'
$npmCache = Join-Path $workspaceRoot '.npm-cache'

$command = @(
    "call `"$vsDevCmd`" -arch=amd64 -host_arch=amd64",
    "set `"PATH=$cargoBin;%PATH%`"",
    "set `"CARGO_HOME=$cargoHome`"",
    "set `"CARGO_TARGET_DIR=$cargoTarget`"",
    "set `"npm_config_cache=$npmCache`"",
    'npm run skill-control:build'
) -join ' && '

Push-Location $repositoryRoot
try {
    & cmd.exe /d /c $command
    if ($LASTEXITCODE -ne 0) {
        throw "Skill Control build failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}
