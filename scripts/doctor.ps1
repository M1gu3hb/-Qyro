$ErrorActionPreference = 'Continue'
$script:Blockers = 0
$script:SimulatedMissing = @(
    ($env:QYRO_DOCTOR_SIMULATE_MISSING -split ',') |
        ForEach-Object { $_.Trim().ToLowerInvariant() } |
        Where-Object { $_ }
)

function Test-SimulatedMissing {
    param([string] $Token)
    return $script:SimulatedMissing -contains $Token.ToLowerInvariant()
}

function Write-Status {
    param(
        [ValidateSet('OK', 'WARNING', 'BLOCKER', 'N/A')]
        [string] $Kind,
        [string] $Label,
        [string] $Detail
    )
    Write-Output "[$Kind] $($Label): $Detail"
}

function Test-CommandTool {
    param(
        [string] $Token,
        [string] $Label,
        [string] $Command,
        [string[]] $Arguments,
        [switch] $Required
    )

    if ((Test-SimulatedMissing $Token) -or -not (Get-Command $Command -ErrorAction SilentlyContinue)) {
        if ($Required) {
            Write-Status 'BLOCKER' $Label 'not found on PATH'
            $script:Blockers++
        }
        else {
            Write-Status 'WARNING' $Label 'not found; optional workflow unavailable'
        }
        return
    }

    $commandOutput = & $Command @Arguments 2>&1
    $detail = [string] ($commandOutput | Select-Object -First 1)
    if ([string]::IsNullOrWhiteSpace($detail)) {
        $detail = 'available'
    }
    Write-Status 'OK' $Label $detail.Trim()
}

Test-CommandTool -Token 'git' -Label 'Git' -Command 'git' -Arguments @('--version') -Required
Test-CommandTool -Token 'flutter' -Label 'Flutter' -Command 'flutter' -Arguments @('--version') -Required
Test-CommandTool -Token 'dart' -Label 'Dart' -Command 'dart' -Arguments @('--version') -Required
Test-CommandTool -Token 'rust' -Label 'Rust' -Command 'rustc' -Arguments @('--version') -Required
Test-CommandTool -Token 'cargo' -Label 'Cargo' -Command 'cargo' -Arguments @('--version') -Required
Test-CommandTool -Token 'fvm' -Label 'FVM' -Command 'fvm' -Arguments @('--version')
Test-CommandTool -Token 'java' -Label 'Java' -Command 'java' -Arguments @('-version')
Test-CommandTool -Token 'cmake' -Label 'CMake' -Command 'cmake' -Arguments @('--version')
Test-CommandTool -Token 'ninja' -Label 'Ninja' -Command 'ninja' -Arguments @('--version')

$androidSdk = if ($env:ANDROID_SDK_ROOT) { $env:ANDROID_SDK_ROOT } else { $env:ANDROID_HOME }
if ((Test-SimulatedMissing 'android-sdk') -or -not $androidSdk -or -not (Test-Path -LiteralPath $androidSdk)) {
    Write-Status 'WARNING' 'Android SDK' 'ANDROID_SDK_ROOT/ANDROID_HOME is not configured'
}
else {
    Write-Status 'OK' 'Android SDK' $androidSdk
}

$ndkRoot = if ($androidSdk) { Join-Path $androidSdk 'ndk' } else { $null }
$ndk = if ($ndkRoot -and (Test-Path -LiteralPath $ndkRoot)) {
    Get-ChildItem -LiteralPath $ndkRoot -Directory -ErrorAction SilentlyContinue |
        Sort-Object Name |
        Select-Object -Last 1
}
if ((Test-SimulatedMissing 'ndk') -or -not $ndk) {
    Write-Status 'WARNING' 'Android NDK' 'not installed under the Android SDK'
}
else {
    Write-Status 'OK' 'Android NDK' $ndk.FullName
}

$isWindowsPlatform = [Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [Runtime.InteropServices.OSPlatform]::Windows
)
$isMacPlatform = [Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [Runtime.InteropServices.OSPlatform]::OSX
)

if ($isMacPlatform) {
    Test-CommandTool -Token 'xcode' -Label 'Xcode' -Command 'xcodebuild' -Arguments @('-version')
    Test-CommandTool -Token 'cocoapods' -Label 'CocoaPods' -Command 'pod' -Arguments @('--version')
    Write-Status 'N/A' 'Visual Studio Build Tools' 'Windows only'
    Write-Status 'N/A' 'Windows SDK' 'Windows only'
}
elseif ($isWindowsPlatform) {
    Write-Status 'N/A' 'Xcode' 'macOS only'
    Write-Status 'N/A' 'CocoaPods' 'macOS only'
    if ((Test-SimulatedMissing 'visual-studio') -or -not (Get-Command 'cl.exe' -ErrorAction SilentlyContinue)) {
        Write-Status 'WARNING' 'Visual Studio Build Tools' 'cl.exe not found on PATH'
    }
    else {
        Write-Status 'OK' 'Visual Studio Build Tools' 'cl.exe available'
    }

    if ($env:WindowsSdkDir) {
        Write-Status 'OK' 'Windows SDK' $env:WindowsSdkDir
    }
    else {
        Write-Status 'WARNING' 'Windows SDK' 'WindowsSdkDir is not configured'
    }
}
else {
    Write-Status 'N/A' 'Xcode' 'macOS only'
    Write-Status 'N/A' 'CocoaPods' 'macOS only'
    Write-Status 'N/A' 'Visual Studio Build Tools' 'Windows only'
    Write-Status 'N/A' 'Windows SDK' 'Windows only'
}

if (Test-SimulatedMissing 'devices') {
    Write-Status 'WARNING' 'Flutter devices' 'device discovery was simulated as unavailable'
}
elseif (Get-Command 'flutter' -ErrorAction SilentlyContinue) {
    $deviceOutput = & flutter devices --machine 2>$null
    if ($LASTEXITCODE -eq 0 -and (($deviceOutput -join [Environment]::NewLine).Contains('"id"'))) {
        Write-Status 'OK' 'Flutter devices' 'at least one device or simulator is discoverable'
    }
    else {
        Write-Status 'WARNING' 'Flutter devices' 'no device or simulator is currently discoverable'
    }
}
else {
    Write-Status 'WARNING' 'Flutter devices' 'Flutter is unavailable'
}

if ($script:Blockers -gt 0) {
    Write-Status 'BLOCKER' 'Doctor summary' "$($script:Blockers) required tool(s) missing"
    exit 1
}

Write-Status 'OK' 'Doctor summary' 'required toolchain is ready'
exit 0
