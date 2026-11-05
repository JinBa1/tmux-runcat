param([Parameter(Mandatory = $true)][string]$FontDirectory, [switch]$KeepTerminalFont)
$ErrorActionPreference = 'Stop'

# Install only this build's manifest, so changing base fonts never installs
# stale files left in the generated directory by an earlier selection.
$fontNames = Get-Content -Raw -LiteralPath (Join-Path $FontDirectory 'font-names.json') | ConvertFrom-Json
$fontFamily = (Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $FontDirectory 'font-family.txt')).Trim()
if (-not $fontFamily -or -not $fontNames.PSObject.Properties['RunCat-Regular.ttf']) {
    throw 'Build the RunCat font family before running this installer.'
}
$fonts = @($fontNames.PSObject.Properties | ForEach-Object {
    Get-Item -LiteralPath (Join-Path $FontDirectory $_.Name)
})
$fontDestination = Join-Path $env:LOCALAPPDATA 'Microsoft\Windows\Fonts'
New-Item -ItemType Directory -Force -Path $fontDestination | Out-Null
$registryPath = 'HKCU:\Software\Microsoft\Windows NT\CurrentVersion\Fonts'
if (-not (Test-Path -LiteralPath $registryPath)) {
    New-Item -Path $registryPath | Out-Null
}
$registryBackup = Join-Path $env:TEMP ('fonts-before-runcat-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.reg')
& reg.exe export 'HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts' $registryBackup /y | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Could not back up font registrations.' }

Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class RunCatFonts {
    [DllImport("gdi32.dll", CharSet = CharSet.Unicode)]
    public static extern int AddFontResourceEx(string path, uint flags, IntPtr reserved);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr SendMessageTimeout(IntPtr window, uint message,
        UIntPtr wParam, IntPtr lParam, uint flags, uint timeout, out UIntPtr result);
}
'@
$fontRegistrations = Get-ItemProperty -Path $registryPath
foreach ($source in $fonts) {
    # Content-based filenames avoid overwriting loaded fonts and avoid making
    # another file on every setup run.
    $hash = (Get-FileHash -LiteralPath $source.FullName).Hash.Substring(0, 12)
    $destination = Join-Path $fontDestination ("runcat-$hash-" + $source.Name)
    if (-not (Test-Path -LiteralPath $destination)) {
        Copy-Item -LiteralPath $source.FullName -Destination $destination
    }
    $registration = $fontNames.($source.Name)
    if (-not $registration) { throw "Missing font name for $($source.Name)" }
    $registered = $fontRegistrations.PSObject.Properties[$registration]
    if ($registered -and $registered.Value -eq $destination) { continue }
    New-ItemProperty -Path $registryPath -Name $registration `
        -Value $destination -PropertyType String -Force | Out-Null
    if ([RunCatFonts]::AddFontResourceEx($destination, 0, [IntPtr]::Zero) -eq 0) {
        throw "Windows could not load $destination"
    }
}
$result = [UIntPtr]::Zero
[RunCatFonts]::SendMessageTimeout([IntPtr]0xffff, 0x001D,
    [UIntPtr]::Zero, [IntPtr]::Zero, 2, 1000, [ref]$result) | Out-Null

if (-not $KeepTerminalFont) {
    & (Join-Path $PSScriptRoot 'configure-windows-terminal.ps1') -FontFamily $fontFamily
}
Write-Output "Installed $($fonts.Count) fonts. Font registry backup: $registryBackup"
if ($KeepTerminalFont) {
    Write-Output "Available: ${fontFamily}."
} else {
    Write-Output "Selected ${fontFamily}: your base font with classic and Pet Cats skins."
}
