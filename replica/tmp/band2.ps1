# Is the white strip on one row real paint, or an artifact of grabbing a GPU-composited
# transparent window with BitBlt?
#
# Three captures of the same, static, untouched overlay, ~350 ms apart. A painted highlight is
# in the same place in all three; a tear moves or vanishes. Each capture also reports the pixel
# values down the middle of the strip's row, so the answer is numbers and not an eye test.
Add-Type -Namespace W -Name Dpi -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
'@
[void][W.Dpi]::SetProcessDpiAwarenessContext([IntPtr](-4))

Add-Type -Namespace W -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
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

$exe = Join-Path $PSScriptRoot '..\..\target\release\shortcut-sheet.exe'
$p = Start-Process -FilePath $exe -ArgumentList '--peek', '"Google Chrome"' -PassThru
Start-Sleep -Seconds 5

# Park the cursor where no row is, so hover cannot be the variable under test.
$h_overlay = [IntPtr]::Zero
foreach ($h in [W.U]::Of([uint32]$p.Id)) {
  $sb = New-Object System.Text.StringBuilder 256
  [void][W.U]::GetWindowTextW($h, $sb, 256)
  if ($sb.ToString() -like 'shortcut-sheet-overlay*' -and [W.U]::IsWindowVisible($h)) { $h_overlay = $h }
}
if ($h_overlay -eq [IntPtr]::Zero) { "no visible overlay"; exit 1 }
$r = New-Object W.U+RECT
[void][W.U]::GetWindowRect($h_overlay, [ref]$r)
"overlay rect     : $($r.Left),$($r.Top) $($r.Right-$r.Left)x$($r.Bottom-$r.Top)"
[void][W.U]::SetCursorPos(4, 4)
Start-Sleep -Milliseconds 500

for ($n = 1; $n -le 3; $n++) {
  # RECT has no Width - it is Left/Top/Right/Bottom, and New-Object with two $nulls throws.
  $bw = $r.Right - $r.Left
  $bh = $r.Bottom - $r.Top
  $full = New-Object System.Drawing.Bitmap $bw, $bh
  $g = [System.Drawing.Graphics]::FromImage($full)
  $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $full.Size)
  # Every row centre: logical 12 pad + 32 title + 30 search + 6 gap + 22 header, then 32 px rows.
  $line = "capture $n rows : "
  for ($row = 0; $row -lt 8; $row++) {
    $y = [int]((122 + $row * 32 + 16) * 2)
    $px = $full.GetPixel(120, [Math]::Min($y, $bh - 1))
    $line += ("[{0}]#{1:X2}{2:X2}{3:X2}a{4} " -f $row, $px.R, $px.G, $px.B, $px.A)
  }
  $line
  $full.Save((Join-Path $PSScriptRoot ("..\screens\band-{0}.png" -f $n)), [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $full.Dispose()
  Start-Sleep -Milliseconds 350
}

# Phase 2: the same overlay with the cursor parked ON row 3 and nothing else changed. If the
# white strip is the hover fill, it appears at row 3 and nowhere else.
"--- cursor over row 3 ---"
$hy = $r.Top + [int]((122 + 3 * 32 + 16) * 2)
[void][W.U]::SetCursorPos($r.Left + 200, $hy)
Start-Sleep -Milliseconds 500
$full = New-Object System.Drawing.Bitmap ($r.Right - $r.Left), ($r.Bottom - $r.Top)
$g = [System.Drawing.Graphics]::FromImage($full)
$g.CopyFromScreen($r.Left, $r.Top, 0, 0, $full.Size)
$line = "hover row px  : "
foreach ($row in 0..7) {
  $y = [int]((122 + $row * 32 + 16) * 2)
  $px = $full.GetPixel(120, $y)
  $line += ("[{0}]#{1:X2}{2:X2}{3:X2}a{4} " -f $row, $px.R, $px.G, $px.B, $px.A)
}
$line
$full.Save((Join-Path $PSScriptRoot '..\screens\band-hover.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $full.Dispose()

Stop-Process -Id $p.Id -ErrorAction SilentlyContinue
