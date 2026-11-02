# Which screenshot method tells the truth about this window?
#
# The overlay is a GL-composited, per-pixel-alpha window. CopyFromScreen grabs the screen
# after DWM has composited it, and on this machine it has been caught returning a frame with
# strips of whatever was behind - which is what looked like an opaque white hover row. The
# only way to know is to grab the same window a second way and compare: PrintWindow with
# PW_RENDERFULLCONTENT asks DWM to render the window itself into a DC.
#
# `-App` is a display name on either shelf, so the roster path (`--peek Slack`: a panel with a
# name and no rows) can be shot exactly like a profile one. `-Tag` prefixes the output files so
# a check does not overwrite the capture it is being compared against.
param(
    [string]$App = 'Google Chrome',
    [string]$Tag = 'grab'
)
Add-Type -Namespace W -Name Dpi -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
'@
[void][W.Dpi]::SetProcessDpiAwarenessContext([IntPtr](-4))

Add-Type -Namespace W -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
public static System.Collections.Generic.List<System.IntPtr> Of(uint wantPid) {
  var list = new System.Collections.Generic.List<System.IntPtr>();
  EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == wantPid) list.Add(h); return true; }, IntPtr.Zero);
  return list;
}
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
'@
Add-Type -AssemblyName System.Windows.Forms, System.Drawing

function RowColors($bmp) {
  $line = ''
  for ($row = 0; $row -lt 8; $row++) {
    $y = [int]((122 + $row * 32 + 16) * 2)
    $px = $bmp.GetPixel(120, [Math]::Min($y, $bmp.Height - 1))
    $line += ("[{0}]#{1:X2}{2:X2}{3:X2} " -f $row, $px.R, $px.G, $px.B)
  }
  $line
}

$exe = Join-Path $PSScriptRoot '..\..\target\release\shortcut-sheet.exe'
$p = Start-Process -FilePath $exe -ArgumentList '--peek', "`"$App`"" -PassThru
Start-Sleep -Seconds 5

$h_overlay = [IntPtr]::Zero
foreach ($h in [W.U]::Of([uint32]$p.Id)) {
  $sb = New-Object System.Text.StringBuilder 256
  [void][W.U]::GetWindowTextW($h, $sb, 256)
  if ($sb.ToString() -like 'shortcut-sheet-overlay*' -and [W.U]::IsWindowVisible($h)) { $h_overlay = $h }
}
if ($h_overlay -eq [IntPtr]::Zero) { "no visible overlay"; exit 1 }
$r = New-Object W.U+RECT
[void][W.U]::GetWindowRect($h_overlay, [ref]$r)
$bw = $r.Right - $r.Left; $bh = $r.Bottom - $r.Top
"overlay rect     : $($r.Left),$($r.Top) ${bw}x${bh}"

# Park the cursor off the panel so nothing is hovered, then shoot the same frame two ways.
[void][W.U]::SetCursorPos(4, 4)
Start-Sleep -Milliseconds 700

$screen = New-Object System.Drawing.Bitmap $bw, $bh
$g = [System.Drawing.Graphics]::FromImage($screen)
$g.CopyFromScreen($r.Left, $r.Top, 0, 0, $screen.Size)
$g.Dispose()
"CopyFromScreen   : " + (RowColors $screen)
$screen.Save((Join-Path $PSScriptRoot ('..\screens\{0}-screen.png' -f $Tag)), [System.Drawing.Imaging.ImageFormat]::Png)

$own = New-Object System.Drawing.Bitmap $bw, $bh
$gc = [System.Drawing.Graphics]::FromImage($own)
$hdc = $gc.GetHdc()
$ok = [W.U]::PrintWindow($h_overlay, $hdc, 2)   # PW_RENDERFULLCONTENT
$gc.ReleaseHdc($hdc)
$gc.Dispose()
"PrintWindow ok={0}: {1}" -f $ok, (RowColors $own)
$own.Save((Join-Path $PSScriptRoot ('..\screens\{0}-window.png' -f $Tag)), [System.Drawing.Imaging.ImageFormat]::Png)

# And once more with the cursor on row 3, which is the case that produced the white strip.
$hy = $r.Top + [int]((122 + 3 * 32 + 16) * 2)
[void][W.U]::SetCursorPos($r.Left + 200, $hy)
Start-Sleep -Milliseconds 700
$screen2 = New-Object System.Drawing.Bitmap $bw, $bh
$g2 = [System.Drawing.Graphics]::FromImage($screen2)
$g2.CopyFromScreen($r.Left, $r.Top, 0, 0, $screen2.Size)
$g2.Dispose()
"hover, screen    : " + (RowColors $screen2)
$screen2.Save((Join-Path $PSScriptRoot ('..\screens\{0}-screen-hover.png' -f $Tag)), [System.Drawing.Imaging.ImageFormat]::Png)

$own2 = New-Object System.Drawing.Bitmap $bw, $bh
$gc2 = [System.Drawing.Graphics]::FromImage($own2)
$hdc2 = $gc2.GetHdc()
[void][W.U]::PrintWindow($h_overlay, $hdc2, 2)
$gc2.ReleaseHdc($hdc2)
$gc2.Dispose()
"hover, window    : " + (RowColors $own2)
$own2.Save((Join-Path $PSScriptRoot ('..\screens\{0}-window-hover.png' -f $Tag)), [System.Drawing.Imaging.ImageFormat]::Png)

Stop-Process -Id $p.Id -ErrorAction SilentlyContinue
