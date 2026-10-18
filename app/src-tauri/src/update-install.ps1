param(
    [Parameter(Mandatory=$true)][string]$Installer,
    [Parameter(Mandatory=$true)][string]$ExpectedHash,
    [Parameter(Mandatory=$true)][string]$App,
    [Parameter(Mandatory=$true)][int]$ParentPid,
    [Parameter(Mandatory=$true)][int]$WatchdogPid
)
$ErrorActionPreference = 'Stop'
$safeToRestart = $false
try {
    foreach ($processId in @($ParentPid, $WatchdogPid)) {
        if ($processId -le 0) { continue }
        try { $process = [Diagnostics.Process]::GetProcessById($processId) }
        catch [ArgumentException] { continue }
        try {
            if (-not $process.WaitForExit(120000)) { throw 'App or watchdog has not exited; update cancelled without stopping it.' }
        } finally { $process.Dispose() }
    }
    $safeToRestart = $true
    $actualHash = (Get-FileHash -LiteralPath $Installer -Algorithm SHA256).Hash
    if ($ExpectedHash -notmatch '^[0-9a-fA-F]{64}$' -or $actualHash -ne $ExpectedHash) { throw 'Installer SHA-256 mismatch; update cancelled.' }
    $setup = Start-Process -FilePath $Installer -ArgumentList '/S' -PassThru -Wait
    if ($setup.ExitCode -ne 0) { throw "Installer refused or failed: $($setup.ExitCode)." }
    'Update installed.' | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'install.log')
} catch {
    $_.Exception.Message | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'install.log')
} finally {
    if ($safeToRestart -and (Test-Path -LiteralPath $App -PathType Leaf)) {
        Start-Process -FilePath $App
    }
}
