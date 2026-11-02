# Is the light band on one row ours, or the desktop showing through?
# Captures the same screen region twice - once with the overlay up, once without - and
# prints the pixel under test at each row. Anything identical in both is not our paint.
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
public static System.Collections.Generic.List<System.IntPtr> Of(uint wantPid) {
  var list = new System.Collections.Generic.List<System.IntPtr>();
  EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == wantPid) list.Add(h); return true; }, IntPtr.Zero);
  return list;
}
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
'@
Add-Type -AssemblyName System.Windows.Forms, System.Drawing

function Grab([System.Drawing.Rectangle]$area, [string]$path) {
  $bmp = New-Object System.Drawing.Bitmap $area.Width, $area.Height
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($area.X, $area.Y, 0, 0, $area.Size)
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose()
  $bmp
}

$exe = Join-Path $PSScriptRoot '..\..\target\release\shortcut-sheet.exe'
$p = Start-Process -FilePath $exe -ArgumentList '--peek' -PassThru
Start-Sleep -Seconds 5

$rect = $null
foreach ($h in [W.U]::Of([uint32]$p.Id)) {
  $sb = New-Object System.Text.StringBuilder 256
  [void][W.U]::GetWindowTextW($h, $sb, 256)
  if ($sb.ToString() -notlike 'shortcut-sheet-overlay*') { continue }
  $r = New-Object W.U+RECT
  [void][W.U]::GetWindowRect($h, [ref]$r)
  if ([W.U]::IsWindowVisible($h)) { $rect = New-Object System.Drawing.Rectangle $r.Left, $r.Top, ($r.Right - $r.Left), ($r.Bottom - $r.Top) }
}
if (-not $rect) { "no visible overlay"; exit 1 }
"overlay at $rect"

$with = Grab $rect (Join-Path $PSScriptRoot 'band-with.png')
Stop-Process -Id $p.Id -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2
$without = Grab $rect (Join-Path $PSScriptRoot 'band-without.png')

# Rows are 32 logical px = 64 physical at 200%, starting under the 32px title and 30px search.
"  y  row |        with overlay |     without overlay | verdict"
foreach ($i in 0..7) {
  $y = 130 + $i * 64
  $a = $with.GetPixel(300, $y)
  $b = $without.GetPixel(300, $y)
  $same = ($a.R -eq $b.R -and $a.G -eq $b.G -and $a.B -eq $b.B)
  "{0,4} {1}  |  #{2:X2}{3:X2}{4:X2} a={5,3}  |  #{6:X2}{7:X2}{8:X2}  | {9}" -f `
    $y, $i, $a.R, $a.G, $a.B, $a.A, $b.R, $b.G, $b.B, $(if ($same) { 'identical - not ours' } else { 'ours' })
}
$with.Dispose(); $without.Dispose()
