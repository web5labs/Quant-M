$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$TempRoot = if ($env:TEMP) { $env:TEMP } else { [System.IO.Path]::GetTempPath() }
$TempDir = Join-Path $TempRoot ("quantm-tui-chat-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $TempDir | Out-Null
$Config = Join-Path $TempDir "quant-m.toml"

try {
    $DebugBin = Join-Path $Root "target\debug\quant-m.exe"
    $ReleaseBin = Join-Path $Root "target\release\quant-m.exe"

    if (Test-Path $DebugBin) {
        $QuantM = @($DebugBin)
    } elseif (Test-Path $ReleaseBin) {
        $QuantM = @($ReleaseBin)
    } else {
        $QuantM = @("cargo", "run", "--quiet", "--")
    }

    @"
Manual Quant-M TUI chat smoke

This opens a throwaway inspect-mode TUI. Try:
  /help
  /stateful should stay ask-inspect text
  /state
  /cost
  /ask does this call a provider?
  /consensus should stay blocked in inspect mode
  /quit

Expected:
  - no provider call
  - no worker write
  - no consensus dry-run artifacts in inspect mode
  - compact terminals show a single-column chat view
  - wide terminals show the evidence rail
"@

    $Command = $QuantM[0]
    $CommandArgs = if ($QuantM.Count -gt 1) { $QuantM[1..($QuantM.Count - 1)] } else { @() }

    & $Command @CommandArgs --config $Config init --non-interactive | Out-Null
    & $Command @CommandArgs --config $Config tui chat --inspect
} finally {
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}
