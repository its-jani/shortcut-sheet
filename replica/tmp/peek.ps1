# Runtime probe for S01. Starts the overlay in peek mode, reads the real Win32 window state,
# and answers the three questions the feature matrix cares about:
#   row 6  is the panel frameless and painted?
#   row 7/8 did showing it steal the foreground from the app behind it?
#   row 14 is it out of the taskbar and alt-tab?
# Evidence for replica/bugs.md and the clone column of replica/features.csv.

# Must be the first thing that touches the desktop. shortcut-sheet is per-monitor-v2 aware, so
# on a 200% display its real window rect is twice what a DPI-unaware probe reads, and a
# screen grab taken in those virtualised coordinates lands on the wrong quarter of the screen.
# Without this the probe "proves" the panel is missing when it is only mislocated.
Add-Type -Namespace W -Name Dpi -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
'@
# DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 is (DPI_AWARENESS_CONTEXT)-4.
[void][W.Dpi]::SetProcessDpiAwarenessContext([IntPtr](-4))

Add-Type -Namespace W -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
[DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int i);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
public static System.Collections.Generic.List<System.IntPtr> TopLevel(uint wantPid) {
  var list = new System.Collections.Generic.List<System.IntPtr>();
  EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == wantPid) list.Add(h); return true; }, IntPtr.Zero);
  return list;
}
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
'@
Add-Type -AssemblyName System.Windows.Forms, System.Drawing

function Title([IntPtr]$h) {
  if ($h -eq [IntPtr]::Zero) { return '<none>' }
  $sb = New-Object System.Text.StringBuilder 256
  [void][W.U]::GetWindowTextW($h, $sb, 256)
  if ($sb.Length -eq 0) { return '<untitled>' }
  $sb.ToString()
}

$before = [W.U]::GetForegroundWindow()
"foreground before  : '{0}'" -f (Title $before)

$exe = Join-Path $PSScriptRoot '..\..\target\release\shortcut-sheet.exe'
$p = Start-Process -FilePath $exe -ArgumentList '--peek' -PassThru
Start-Sleep -Seconds 5

$after = [W.U]::GetForegroundWindow()
"foreground during  : '{0}'" -f (Title $after)
"focus not stolen   : {0}" -f ($before -eq $after)

$rect = $null
$h_overlay = [IntPtr]::Zero
foreach ($h in [W.U]::TopLevel([uint32]$p.Id)) {
  $title = Title $h
  if ($title -notlike 'shortcut-sheet-overlay*') { continue }
  $cn = New-Object System.Text.StringBuilder 256
  [void][W.U]::GetClassNameW($h, $cn, 256)
  $st = [W.U]::GetWindowLong($h, -16)
  $ex = [W.U]::GetWindowLong($h, -20)
  $r = New-Object W.U+RECT
  [void][W.U]::GetWindowRect($h, [ref]$r)
  "overlay window   : class={0}" -f $cn.ToString()
  "  visible={0}  rect={1},{2} {3}x{4}" -f [W.U]::IsWindowVisible($h), $r.Left, $r.Top, ($r.Right - $r.Left), ($r.Bottom - $r.Top)
  "  physical size    : {0}x{1} px (the logical panel is 344 wide, so 2.0 means a 200% display)" -f ($r.Right - $r.Left), ($r.Bottom - $r.Top)
  "  style=0x{0:X8} exstyle=0x{1:X8}" -f $st, $ex
  "  NOACTIVATE={0} TOOLWINDOW={1} TOPMOST={2} APPWINDOW={3}" -f `
    (($ex -band 0x08000000) -ne 0), (($ex -band 0x00000080) -ne 0), `
    (($ex -band 0x00000008) -ne 0), (($ex -band 0x00040000) -ne 0)
  # WS_CAPTION stays set by design in winit 0.30 - it makes a window frameless by answering
  # WM_NCCALCSIZE, not by clearing the style bit. Only the screenshot can settle row 6.
  "  WS_CAPTION={0} (winit keeps this; framelessness is WM_NCCALCSIZE)" -f (($st -band 0x00C00000) -ne 0)
  if ([W.U]::IsWindowVisible($h)) { $rect = $r; $h_overlay = $h }
}

if ($rect) {
  # PrintWindow(PW_RENDERFULLCONTENT), not CopyFromScreen. The overlay is a GL-composited
  # window with per-pixel alpha, and a screen grab of it has twice come back with strips of
  # the page behind in place of the panel - which read as "an opaque white hover row" and as
  # "the panel does not paint". Both were the grab, not the app. Asking DWM to render the
  # window itself is the version that agrees with what the user sees.
  $bw = $rect.Right - $rect.Left
  $bh = $rect.Bottom - $rect.Top
  $crop = New-Object System.Drawing.Bitmap $bw, $bh
  $g = [System.Drawing.Graphics]::FromImage($crop)
  $hdc = $g.GetHdc()
  $ok = [W.U]::PrintWindow($h_overlay, $hdc, 2)
  $g.ReleaseHdc($hdc)
  $out = (Join-Path $PSScriptRoot '..\screens\clone-S01-peek.png')
  $crop.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
  $crop.Dispose(); $g.Dispose()
  "screenshot       : $out ($bw x $bh px, PrintWindow=$ok)"
} else { "screenshot       : no visible overlay window to capture" }

Get-Process -Id $p.Id -ErrorAction SilentlyContinue | ForEach-Object {
  "memory           : {0:N1} MB working set, {1:N1} MB private" -f ($_.WorkingSet64/1MB), ($_.PrivateMemorySize64/1MB)
}

# The peek deadline closes the app through the same request_quit/try_close path the tray's
# "Quit shortcut-sheet" item uses, so waiting it out proves the only exit works.
Start-Sleep -Seconds 24
$alive = [bool](Get-Process -Id $p.Id -ErrorAction SilentlyContinue)
"quit on deadline   : {0} (True = the app exited by itself)" -f (-not $alive)
Stop-Process -Id $p.Id -ErrorAction SilentlyContinue
"--- log (last 20 lines) ---"
Get-ChildItem "$env:LOCALAPPDATA\shortcut-sheet\logs" -ErrorAction SilentlyContinue |
  ForEach-Object { Get-Content $_.FullName -Tail 20 }
