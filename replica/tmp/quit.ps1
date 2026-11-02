# Does the app leave when it is asked to? Polls once a second and reports the elapsed time,
# so "slow" and "never" are different answers. Drives the same request_quit path the tray's
# Quit item uses.
$exe = Join-Path $PSScriptRoot '..\..\target\release\shortcut-sheet.exe'
$t0 = Get-Date
$proc = Start-Process -FilePath $exe -ArgumentList '--peek' -PassThru
$target = $proc.Id
while ($true) {
  Start-Sleep -Seconds 1
  $elapsed = (New-TimeSpan -Start $t0 -End (Get-Date)).Seconds
  $still = Get-Process -Id $target -ErrorAction SilentlyContinue
  if ($null -eq $still) {
    "exited on its own after $elapsed s"
    break
  }
  if ($elapsed -ge 45) {
    "STILL RUNNING after $elapsed s - it did not quit"
    break
  }
}
Stop-Process -Id $target -ErrorAction SilentlyContinue
