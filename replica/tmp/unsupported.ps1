# Does an app in NEITHER shelf really resolve as Unsupported, live?
#
# F01-E7 was only ever proven by a unit test against a mocked exe name. This drives it for real:
# open a window that no profile and no roster entry claims, put it in front, run `--detect`
# (which reads the live foreground window and installs nothing), and print what the resolution
# said. System Information is the subject because it is a plain Win32 process that owns its own
# window - unlike Store Notepad, whose window belongs to a different pid than the stub launches -
# it changes nothing on the machine, and it is closed at the end whether the run worked or not.
Add-Type -Namespace W -Name Dpi -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
'@
[void][W.Dpi]::SetProcessDpiAwarenessContext([IntPtr](-4))

Add-Type -Namespace W -Name F -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
public static System.Collections.Generic.List<System.IntPtr> Of(uint wantPid) {
  var list = new System.Collections.Generic.List<System.IntPtr>();
  EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == wantPid && IsWindowVisible(h)) list.Add(h); return true; }, IntPtr.Zero);
  return list;
}
'@

$exe = Join-Path $PSScriptRoot '..\..\target\release\shortcut-sheet.exe'
$subject = Start-Process msinfo32.exe -PassThru
try {
  # msinfo32 shows a splash for a moment; wait for the window it actually wants you to see.
  $h = [IntPtr]::Zero
  for ($try = 0; $try -lt 20 -and $h -eq [IntPtr]::Zero; $try++) {
    Start-Sleep -Milliseconds 500
    $visible = [W.F]::Of([uint32]$subject.Id)
    foreach ($c in $visible) {
      $sb = New-Object System.Text.StringBuilder 256
      [void][W.F]::GetWindowTextW($c, $sb, 256)
      if ($sb.Length -gt 0) { $h = $c }
    }
  }
  if ($h -eq [IntPtr]::Zero) { throw "no visible window belongs to msinfo32 pid $($subject.Id)" }
  [void][W.F]::SetForegroundWindow($h)
  Start-Sleep -Milliseconds 400
  $sb = New-Object System.Text.StringBuilder 256
  [void][W.F]::GetWindowTextW([W.F]::GetForegroundWindow(), $sb, 256)
  "foreground now: '{0}'" -f $sb.ToString()
  & $exe --detect
} finally {
  Stop-Process -Id $subject.Id -Force -ErrorAction SilentlyContinue
  'system information closed'
}
