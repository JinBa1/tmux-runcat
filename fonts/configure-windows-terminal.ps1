param(
    [string]$SettingsPath = (Join-Path $env:LOCALAPPDATA 'Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json'),
    [string]$FontFamily = (Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $PSScriptRoot 'font-family.txt')).Trim()
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'windows-terminal-settings.ps1')
if ([string]::IsNullOrWhiteSpace($FontFamily)) { throw 'FontFamily must not be empty.' }
if (Test-Path -LiteralPath $settingsPath) {
    $config = Read-RunCatTerminalSettings $settingsPath
    if (-not $config.profiles.PSObject.Properties['defaults']) {
        $config.profiles | Add-Member -NotePropertyName defaults -NotePropertyValue ([pscustomobject]@{})
    }
    $targets = @($config.profiles.defaults)
    $previousDefault = $config.profiles.defaults.font.face
    foreach ($profile in $config.profiles.list) {
        if ($profile.source -match '^(Microsoft\.WSL|Windows\.Terminal\.Wsl)$' -or
            $profile.font.face -match '(^JetBrainsMono Nerd Font|RunCat)' -or
            ($previousDefault -and $profile.font.face -eq $previousDefault)) {
            $targets += $profile
        }
    }
    $changed = $false
    foreach ($profile in $targets) {
        if (-not $profile.PSObject.Properties['font']) {
            $profile | Add-Member -NotePropertyName font -NotePropertyValue ([pscustomobject]@{})
        }
        if ($profile.font.face -ne $fontFamily) {
            $profile.font | Add-Member -NotePropertyName face -NotePropertyValue $fontFamily -Force
            $changed = $true
        }
    }
    if ($changed) {
        $backup = $settingsPath + '.runcat-' + (Get-Date -Format 'yyyyMMdd-HHmmss-fff') + '.bak'
        Copy-Item -LiteralPath $settingsPath -Destination $backup
        $updated = $config | ConvertTo-Json -Depth 100
        [IO.File]::WriteAllText($settingsPath, $updated, (New-Object Text.UTF8Encoding $false))
        Write-Output "Windows Terminal settings backup: $backup"
    }
}
