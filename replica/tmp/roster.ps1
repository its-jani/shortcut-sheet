# Does a roster app - one the target knows and this build ships no rows for - really produce a
# panel rather than a silence or a crash? `--peek` is the only way to reach S01 from a script, and
# `knows()` accepts both shelves, so this exercises the Context::Empty path end to end.
$env:RUST_LOG = 'shortcut_sheet=debug'
$exe = Join-Path $PSScriptRoot '..\..\target\release\shortcut-sheet.exe'
$log = Get-Item (Join-Path $env:LOCALAPPDATA 'shortcut-sheet\logs\shortcut-sheet.log.*') |
  Sort-Object LastWriteTime | Select-Object -Last 1
$mark = (Get-Content $log.FullName).Count

$p = Start-Process -FilePath $exe -ArgumentList '--peek', '"Slack"' -PassThru
Start-Sleep -Seconds 4
"process alive      : $(-not $p.HasExited)"
Stop-Process -Id $p.Id -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 400

"--- this run's log ---"
Get-Content $log.FullName | Select-Object -Skip ($mark - 1) |
  Select-String -Pattern 'shown|peek|panic|warn|error|stopped' |
  ForEach-Object { $_.Line }
