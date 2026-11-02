# The idle footprint, measured the way BUG-006 states it: launched normally (hook installed,
# tray icon up, nothing painted, no key held), left alone, and read.
#
# Peek mode is deliberately not measured here - peek.ps1 already reports memory with the panel
# painted, and the two numbers answer different questions. "Resident and idle" is the one the
# brief set a budget for.
param([int]$Seconds = 12)
$p = Start-Process -FilePath (Join-Path $PSScriptRoot '..\..\target\release\shortcut-sheet.exe') -PassThru
Start-Sleep -Seconds $Seconds
try {
  $pr = Get-Process -Id $p.Id
  "idle memory: {0:N1} MB working set, {1:N1} MB private, {2} threads" -f ($pr.WorkingSet64 / 1MB), ($pr.PrivateMemorySize64 / 1MB), $pr.Threads.Count
} finally {
  # A hard stop, so the non-blocking log tail is lost - which is fine: this probe reads the
  # process, not the log. Quitting through the tray path would need a window handle to post to.
  Stop-Process -Id $p.Id -ErrorAction SilentlyContinue
}
