$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

function Require-Text {
    param([string]$Path, [string]$Text)
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "[FAIL] Missing $Path"
    }
    $content = Get-Content -LiteralPath $Path -Raw
    if (-not $content.Contains($Text)) {
        throw "[FAIL] Expected $Path to contain: $Text"
    }
}

function Reject-Text {
    param([string]$Path, [string]$Text)
    if ((Test-Path -LiteralPath $Path) -and
        (Get-Content -LiteralPath $Path -Raw).Contains($Text)) {
        throw "[FAIL] Expected $Path not to contain: $Text"
    }
}

$android = Join-Path $repoRoot 'apps/qyro/android/app/src/main/res'
Require-Text (Join-Path $android 'values/colors.xml') '<color name="qyro_background">#03070D</color>'
Require-Text (Join-Path $android 'drawable/launch_background.xml') '@color/qyro_background'
Require-Text (Join-Path $android 'drawable-v21/launch_background.xml') '@color/qyro_background'
Require-Text (Join-Path $android 'values/styles.xml') '@drawable/launch_background'
Require-Text (Join-Path $android 'values-night/styles.xml') '@drawable/launch_background'
Reject-Text (Join-Path $android 'drawable/launch_background.xml') '@android:color/white'
Reject-Text (Join-Path $android 'drawable-v21/launch_background.xml') '?android:colorBackground'
Reject-Text (Join-Path $android 'values/styles.xml') 'Theme.Light'

$ios = Join-Path $repoRoot 'apps/qyro/ios/Runner/Base.lproj/LaunchScreen.storyboard'
Require-Text $ios 'red="0.01176470588"'
Require-Text $ios 'green="0.02745098039"'
Require-Text $ios 'blue="0.05098039216"'
Reject-Text $ios 'image="LaunchImage"'
Reject-Text $ios 'red="1" green="1" blue="1"'

# Interface Builder refuses to open a storyboard whose document element omits
# toolsVersion, and fails the whole iOS build with "com.apple.InterfaceBuilder
# error -1" long before any Dart code runs. Assert the loadable structure here so
# a non-macOS runner catches it too.
try {
    $storyboard = [xml](Get-Content -LiteralPath $ios -Raw)
} catch {
    throw "[FAIL] $ios is not well-formed XML: $($_.Exception.Message)"
}

$documentElement = $storyboard.DocumentElement
if ($documentElement.Name -ne 'document') {
    throw "[FAIL] $ios root element must be <document>"
}

foreach ($attribute in @('toolsVersion', 'targetRuntime', 'initialViewController')) {
    if ([string]::IsNullOrEmpty($documentElement.GetAttribute($attribute))) {
        throw "[FAIL] $ios <document> must declare $attribute or Interface Builder cannot open it"
    }
}

if ($documentElement.GetAttribute('launchScreen') -ne 'YES') {
    throw "[FAIL] $ios must stay a launch screen"
}

$toolsVersion = $documentElement.GetAttribute('toolsVersion')
foreach ($capability in $storyboard.SelectNodes('//capability')) {
    $minimum = $capability.GetAttribute('minToolsVersion')
    if (-not [string]::IsNullOrEmpty($minimum) -and
        [string]::Compare($minimum, $toolsVersion, [System.StringComparison]::Ordinal) -gt 0) {
        throw "[FAIL] $ios declares capability '$($capability.GetAttribute('name'))' above its toolsVersion"
    }
}

$windows = Join-Path $repoRoot 'apps/qyro/windows/runner/win32_window.cpp'
Require-Text $windows 'RGB(3, 7, 13)'
Require-Text $windows 'window_class.hbrBackground ='
Reject-Text $windows 'window_class.hbrBackground = 0;'

Write-Output '[PASS] Native launch surfaces use the Qyro dark background'
