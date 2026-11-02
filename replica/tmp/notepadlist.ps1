# Read-only: which Notepad windows are on the desktop, and what do they say they hold?
#
# The first attempt at unsupported.ps1 launched notepad.exe and then killed the stub pid. Store
# Notepad answers that by leaving the real window - which may or may not be the user's own - so
# before anything else touches Notepad this lists every window with its title and its process id.
# Nothing here closes or sends input to any of them.
Add-Type -Namespace W -Name Dpi -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
'@
[void][W.Dpi]::SetProcessDpiAwarenessContext([IntPtr](-4))

Add-Type -Namespace W -Name F -MemberDefinition @'
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
public static System.Collections.Generic.List<System.IntPtr> All() {
  var list = new System.Collections.Generic.List<System.IntPtr>();
  EnumWindows((h, l) => { if (IsWindowVisible(h)) list.Add(h); return true; }, IntPtr.Zero);
  return list;
}
'@

$byPid = @{}
foreach ($p in [System.Diagnostics.Process]::GetProcesses()) {
  try { $byPid[$p.Id] = $p.ProcessName } catch { }
}
foreach ($h in [W.F]::All()) {
  $sb = New-Object System.Text.StringBuilder 512
  [void][W.F]::GetWindowTextW($h, $sb, 512)
  $title = $sb.ToString()
  if ($title -notmatch 'Notepad') { continue }
  $pid2 = [uint32]0
  [void][W.F]::GetWindowThreadProcessId($h, [ref]$pid2)
  "hwnd={0}  pid={1} ({2})  title='{3}'" -f $h, $pid2, $byPid[[int]$pid2], $title
}
"notepad processes: " + (([System.Diagnostics.Process]::GetProcessesByName('Notepad') | ForEach-Object { $_.Id }) -join ', ')
