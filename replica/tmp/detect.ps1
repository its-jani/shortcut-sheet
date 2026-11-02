# Live check of the foreground-process path (must-have row 4/5): the exe the app resolves
# for the window in front, and the profile it maps to. --detect opens no window and installs
# no hook, so it can run beside a live instance. Two readings: whatever is in front now, and
# again after a File Explorer window is brought up - explorer.exe is in the dataset, so the
# second one has to come back Known.
Add-Type -Namespace W -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint msg, IntPtr w, IntPtr l);
'@

$exe = Join-Path $PSScriptRoot '..\..\target\release\shortcut-sheet.exe'

function Probe([string]$label) {
  $out = Join-Path $env:TEMP ("ss-detect-{0}.txt" -f $label)
  Remove-Item $out -ErrorAction SilentlyContinue
  $fg = [W.U]::GetForegroundWindow()
  $pid2 = 0
  [void][W.U]::GetWindowThreadProcessId($fg, [ref]$pid2)
  $sb = New-Object System.Text.StringBuilder 256
  [void][W.U]::GetWindowTextW($fg, $sb, 256)
  "in front now      : {0} (pid {1})" -f $sb.ToString(), $pid2
  Start-Process -FilePath $exe -ArgumentList '--detect' -Wait -NoNewWindow `
    -RedirectStandardOutput $out
  "detect [{0}]  : {1}" -f $label, (Get-Content $out -Raw).Trim()
}

Probe 'a'

# Bring a known app forward: a fresh Explorer window takes the foreground on its own. The
# window is closed by handle afterwards - never by process name, since explorer.exe is the
# desktop shell and killing it takes the taskbar down with it.
$before = [W.U]::GetForegroundWindow()
Start-Process explorer.exe -ArgumentList $env:TEMP
Start-Sleep -Seconds 3
Probe 'b'

$now = [W.U]::GetForegroundWindow()
if ($now -ne $before) { [void][W.U]::PostMessageW($now, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) }
