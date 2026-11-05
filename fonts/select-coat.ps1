param(
    [string]$Coat = 'random',
    [string]$SettingsPath = (Join-Path $env:LOCALAPPDATA 'Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json'),
    [string]$BaseFamily = 'JetBrainsMono'
)
$ErrorActionPreference = 'Stop'

function Select-RunCatCoat(
    [string]$Requested, [System.Collections.IDictionary]$Families,
    [string[]]$Installed, [string]$CurrentFamily
) {
    $requested = $Requested.Trim().ToLowerInvariant()
    $available = @($Families.Keys | Where-Object { $Families[$_] -in $Installed })
    if ($requested -ne 'random') {
        if (-not $Families.Contains($requested)) { throw "Unknown RunCat coat: $requested" }
        if ($requested -notin $available) { throw "Coat is not installed: $requested. Run fonts/setup.sh --fur $requested first." }
        return $requested
    }
    if ($available.Count -eq 0) { throw 'No RunCat color coats are installed. Install a colored coat first.' }
    if ($available.Count -gt 1) {
        $available = @($available | Where-Object { $Families[$_] -ne $CurrentFamily })
    }
    return $available | Get-Random
}

# Dot sourcing loads the pure chooser for tests without reading or changing profiles.
if ($MyInvocation.InvocationName -eq '.') { return }
. (Join-Path $PSScriptRoot 'windows-terminal-settings.ps1')
# Multiple attached clients/servers share the same Windows Terminal settings.
# Serialize read, choice and write so each event sees the previous choice.
$mutex = [Threading.Mutex]::new($false, 'Local\RunCat-CoatSelection')
$locked = $false
try {
    try { $locked = $mutex.WaitOne(10000) } catch [Threading.AbandonedMutexException] { $locked = $true }
    if (-not $locked) { throw 'Another RunCat coat selection is still running.' }
    if (-not (Test-Path -LiteralPath $SettingsPath)) { throw 'Windows Terminal settings were not found.' }
    $config = Read-RunCatTerminalSettings $SettingsPath
    $families = [ordered]@{}
    Get-Content -LiteralPath (Join-Path $PSScriptRoot 'pet-cats/coats.tsv') | ForEach-Object {
        if ($_ -and -not $_.StartsWith('#')) {
            $name, $suffix = $_.Split("`t")
            $families[$name] = "$BaseFamily RunCat $suffix v1"
        }
    }
    Add-Type -AssemblyName PresentationCore
    $installed = @([Windows.Media.Fonts]::SystemFontFamilies | ForEach-Object { $_.Source })
    $selected = Select-RunCatCoat $Coat $families $installed $config.profiles.defaults.font.face
    & (Join-Path $PSScriptRoot 'configure-windows-terminal.ps1') -SettingsPath $SettingsPath `
        -FontFamily $families[$selected] | Write-Verbose
    Write-Output $selected
} finally {
    if ($locked) { $mutex.ReleaseMutex() }
    $mutex.Dispose()
}
