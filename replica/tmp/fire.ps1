# Does a click on a row actually fire the combo? Peek mode puts S01 on screen without a key
# hold; a real mouse click on row 0 drives Overlay::draw -> Action::Fire -> Sheet::fire ->
# SendInput. RUST_LOG=debug so the "fired" line is written. The chord itself is Ctrl+N, which
# opens a new window in whatever is behind - that is the point, and it is what the user sees.
Add-Type -Namespace W -Name Dpi -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
'@
[void][W.Dpi]::SetProcessDpiAwarenessContext([IntPtr](-4))

Add-Type -Namespace W -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
[DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, IntPtr extra);
public static System.Collections.Generic.List<System.IntPtr> Of(uint wantPid) {
  var list = new System.Collections.Generic.List<System.IntPtr>();
  EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == wantPid) list.Add(h); return true; }, IntPtr.Zero);
  return list;
}
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
'@

$env:RUST_LOG = 'debug'
$exe = Join-Path $PSScriptRoot '..\..\target\release\shortcut-sheet.exe'
$p = Start-Process -FilePath $exe -ArgumentList '--peek' -PassThru
Start-Sleep -Seconds 5

$pt = $null
foreach ($h in [W.U]::Of([uint32]$p.Id)) {
  $sb = New-Object System.Text.StringBuilder 256
  [void][W.U]::GetWindowTextW($h, $sb, 256)
  if ($sb.ToString() -notlike 'shortcut-sheet-overlay*') { continue }
  $r = New-Object W.U+RECT
  [void][W.U]::GetWindowRect($h, [ref]$r)
  if (-not [W.U]::IsWindowVisible($h)) { continue }
  # Row 0 spans logical y 122..154 (12 pad + 32 title + 30 search + 6 gap + 22 header), so
  # logical 130 = 260 physical px at 200%. 300 px in is the middle of the name column.
  $pt = @{ X = $r.Left + 300; Y = $r.Top + 260 }
  "overlay at         : $($r.Left),$($r.Top) $($r.Right - $r.Left)x$($r.Bottom - $r.Top)"
}
if (-not $pt) { "no visible overlay"; exit 1 }

"clicking           : $($pt.X),$($pt.Y)"
[void][W.U]::SetCursorPos($pt.X, $pt.Y)
Start-Sleep -Milliseconds 300
[W.U]::mouse_event(0x0002, 0, 0, 0, [IntPtr]::Zero)   # LEFTDOWN
Start-Sleep -Milliseconds 80
[W.U]::mouse_event(0x0004, 0, 0, 0, [IntPtr]::Zero)   # LEFTUP
Start-Sleep -Seconds 2
Stop-Process -Id $p.Id -ErrorAction SilentlyContinue

"--- lines mentioning fire or focus ---"
Get-ChildItem "$env:LOCALAPPDATA\shortcut-sheet\logs" -ErrorAction SilentlyContinue |
  ForEach-Object { Get-Content $_.FullName } |
  Select-String -Pattern 'fired|SendInput|focus|error|warn' |
  Select-Object -Last 12 | ForEach-Object { $_.Line }
