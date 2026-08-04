param(
    [string] $RepoRoot = (Join-Path $PSScriptRoot '..'),
    [switch] $SkipDependencies
)

$ErrorActionPreference = 'Stop'
$script:Blockers = 0

function Write-Status {
    param(
        [ValidateSet('OK', 'N/A', 'BLOCKER')]
        [string] $Kind,
        [string] $Label,
        [string] $Detail
    )
    Write-Output "[$Kind] $($Label): $Detail"
}

function Invoke-Step {
    param(
        [string] $Label,
        [scriptblock] $Action
    )

    try {
        & $Action
        if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) {
            throw "command exited with $LASTEXITCODE"
        }
        Write-Status 'OK' $Label 'completed'
    }
    catch {
        Write-Status 'BLOCKER' $Label $_.Exception.Message
        $script:Blockers++
    }
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    Write-Status 'BLOCKER' 'Repository' "$RepoRoot does not exist"
    exit 1
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
$flutterRoot = Join-Path $RepoRoot 'apps/qyro'

if ($SkipDependencies) {
    Write-Status 'N/A' 'Rust dependencies' 'skipped by request'
    Write-Status 'N/A' 'Flutter dependencies' 'skipped by request'
}
else {
    $cargoManifest = Join-Path $RepoRoot 'Cargo.toml'
    if (Test-Path -LiteralPath $cargoManifest) {
        Invoke-Step 'Rust dependencies' { & cargo fetch --manifest-path $cargoManifest }
    }
    else {
        Write-Status 'BLOCKER' 'Rust dependencies' 'Cargo.toml not found'
        $script:Blockers++
    }

    $pubspec = Join-Path $flutterRoot 'pubspec.yaml'
    if (Test-Path -LiteralPath $pubspec) {
        Invoke-Step 'Flutter dependencies' {
            Push-Location $flutterRoot
            try {
                & flutter pub get
            }
            finally {
                Pop-Location
            }
        }
    }
    else {
        Write-Status 'BLOCKER' 'Flutter dependencies' 'apps/qyro/pubspec.yaml not found'
        $script:Blockers++
    }
}

function Copy-ConfigIfMissing {
    param(
        [string] $Label,
        [string] $Example,
        [string] $Target
    )

    if (Test-Path -LiteralPath $Target) {
        Write-Status 'OK' $Label 'preserved existing local configuration'
    }
    elseif (Test-Path -LiteralPath $Example) {
        Copy-Item -LiteralPath $Example -Destination $Target
        Write-Status 'OK' $Label 'created local configuration from example'
    }
    else {
        Write-Status 'BLOCKER' $Label 'example file missing'
        $script:Blockers++
    }
}

$configRoot = Join-Path $RepoRoot 'config'
New-Item -ItemType Directory -Path $configRoot -Force | Out-Null
Copy-ConfigIfMissing -Label 'Branding config' -Example (Join-Path $configRoot 'branding.example.json') -Target (Join-Path $configRoot 'branding.json')
Copy-ConfigIfMissing -Label 'Feature config' -Example (Join-Path $configRoot 'features.example.json') -Target (Join-Path $configRoot 'features.json')

$logo = Join-Path $flutterRoot 'assets/brand/qyro-logo.png'
if (Test-Path -LiteralPath $logo) {
    Write-Status 'OK' 'Brand assets' 'Qyro logo is ready'
}
else {
    Write-Status 'BLOCKER' 'Brand assets' 'apps/qyro/assets/brand/qyro-logo.png is missing'
    $script:Blockers++
}

$ffigenConfig = Join-Path $flutterRoot 'ffigen.yaml'
if (Test-Path -LiteralPath $ffigenConfig) {
    Invoke-Step 'FFI bindings' {
        Push-Location $flutterRoot
        try {
            & dart run ffigen --config ffigen.yaml
        }
        finally {
            Pop-Location
        }
    }
}
else {
    Write-Status 'N/A' 'FFI bindings' 'no ffigen configuration is present yet'
}

$pubspecPath = Join-Path $flutterRoot 'pubspec.yaml'
$hasBuildRunner = (Test-Path -LiteralPath $pubspecPath) -and
    (Select-String -LiteralPath $pubspecPath -Pattern '^\s*build_runner:' -Quiet)
if ($hasBuildRunner) {
    Invoke-Step 'Code generation' {
        Push-Location $flutterRoot
        try {
            & dart run build_runner build --delete-conflicting-outputs
        }
        finally {
            Pop-Location
        }
    }
}
else {
    Write-Status 'N/A' 'Code generation' 'no build_runner configuration is present'
}

if ($script:Blockers -gt 0) {
    Write-Status 'BLOCKER' 'Bootstrap summary' "$($script:Blockers) step(s) failed"
    exit 1
}

Write-Status 'OK' 'Bootstrap summary' 'workspace is prepared'
exit 0
