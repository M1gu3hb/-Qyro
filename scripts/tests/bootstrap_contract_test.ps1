$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$bootstrap = Join-Path $repoRoot 'scripts/bootstrap.ps1'

if (-not (Test-Path -LiteralPath $bootstrap)) {
    throw "Expected $bootstrap to exist."
}

$workspace = Join-Path ([IO.Path]::GetTempPath()) "qyro-bootstrap-$([Guid]::NewGuid())"
try {
    New-Item -ItemType Directory -Path (Join-Path $workspace 'config') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $workspace 'apps/qyro/assets/brand') -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $workspace 'config/branding.example.json') -Value '{"app":"example"}'
    Set-Content -LiteralPath (Join-Path $workspace 'config/features.example.json') -Value '{"feature":false}'
    Set-Content -LiteralPath (Join-Path $workspace 'apps/qyro/assets/brand/qyro-logo.png') -Value 'asset'

    $output = & pwsh -NoProfile -File $bootstrap -RepoRoot $workspace -SkipDependencies 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "bootstrap.ps1 failed: $($output -join [Environment]::NewLine)"
    }

    $branding = Join-Path $workspace 'config/branding.json'
    $features = Join-Path $workspace 'config/features.json'
    if (-not (Test-Path -LiteralPath $branding) -or -not (Test-Path -LiteralPath $features)) {
        throw 'bootstrap.ps1 did not create local configurations.'
    }

    $brandingExample = Get-Content -LiteralPath (Join-Path $workspace 'config/branding.example.json') -Raw
    $featuresExample = Get-Content -LiteralPath (Join-Path $workspace 'config/features.example.json') -Raw
    if ((Get-Content -LiteralPath $branding -Raw) -ne $brandingExample) {
        throw 'Branding config does not match its example.'
    }
    if ((Get-Content -LiteralPath $features -Raw) -ne $featuresExample) {
        throw 'Feature config does not match its example.'
    }

    $text = $output -join [Environment]::NewLine
    foreach ($expected in @(
        '[N/A] Rust dependencies',
        '[N/A] Flutter dependencies',
        '[OK] Branding config',
        '[OK] Feature config',
        '[OK] Brand assets',
        '[N/A] FFI bindings',
        '[N/A] Code generation'
    )) {
        if (-not $text.Contains($expected)) {
            throw "Expected output to contain '$expected'. Actual output:$([Environment]::NewLine)$text"
        }
    }

    Set-Content -LiteralPath $branding -Value '{"app":"custom"}'
    $secondOutput = & pwsh -NoProfile -File $bootstrap -RepoRoot $workspace -SkipDependencies 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Second bootstrap.ps1 run failed: $($secondOutput -join [Environment]::NewLine)"
    }
    if ((Get-Content -LiteralPath $branding -Raw).Trim() -ne '{"app":"custom"}') {
        throw 'bootstrap.ps1 overwrote a user configuration.'
    }
}
finally {
    if (Test-Path -LiteralPath $workspace) {
        Remove-Item -LiteralPath $workspace -Recurse -Force
    }
}
