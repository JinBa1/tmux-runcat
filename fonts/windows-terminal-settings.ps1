function Read-RunCatTerminalSettings([string]$Path) {
    $settings = [IO.File]::ReadAllText($Path)
    # Keep quoted strings intact while removing JSONC comments and trailing commas.
    $json = [regex]::Replace($settings, '("(?:\\.|[^"\\])*")|(/\*[\s\S]*?\*/|//[^\r\n]*)', {
        param($match)
        if ($match.Groups[1].Success) { return $match.Value }
        return ' '
    })
    $json = [regex]::Replace($json, '("(?:\\.|[^"\\])*")|,(\s*[}\]])', {
        param($match)
        if ($match.Groups[1].Success) { return $match.Value }
        return $match.Groups[2].Value
    })
    return $json | ConvertFrom-Json
}
