# The light half of "light and dark theme following the system", which until now had never been
# rendered by anything.
#
# There is no tray settings dialog, so the only way to see the LIGHT palette on purpose is the
# override in settings.json - which is also the thing that makes `theme` a setting instead of a
# string in a struct (BUG-010). The user's own file is put back exactly as it was found,
# including "there was no file there".
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

$dir = Join-Path $env:LOCALAPPDATA 'shortcut-sheet'
$settings = Join-Path $dir 'settings.json'
$backup = Join-Path $env:TEMP 'ss-settings-backup.json'
$hadFile = Test-Path $settings
if ($hadFile) { Copy-Item $settings $backup -Force }

function Shoot([IntPtr]$h, [string]$name) {
  $r = New-Object W.U+RECT
  [void][W.U]::GetWindowRect($h, [ref]$r)
  $w = $r.Right - $r.Left
  $hh = $r.Bottom - $r.Top
  $bmp = New-Object System.Drawing.Bitmap $w, $hh
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $hdc = $g.GetHdc()
  [void][W.U]::PrintWindow($h, $hdc, 2)
  $g.ReleaseHdc($hdc)
  $out = Join-Path $PSScriptRoot ("..\screens\{0}.png" -f $name)
  $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
  # Three readings decide it. The surface and a command-name glyph say which palette ran. The
  # third is the search field, and it is a *scan* rather than one pixel: the field is a 30-point
  # pill and a single guessed row landed just under it, reporting the panel colour and hiding
  # the thing being checked. The modal colour of the band is the field's fill.
  $surface = $bmp.GetPixel(60, [int]($hh * 0.82))
  $text = $bmp.GetPixel(60, [int]($hh * 0.31))
  $fx = [int]($w * 0.44)
  $tally = @{}
  for ($y = 80; $y -lt 150; $y++) {
    $c = $bmp.GetPixel($fx, $y)
    $key = '{0:X2}{1:X2}{2:X2}' -f $c.R, $c.G, $c.B
    $tally[$key] = 1 + ($tally[$key] ?? 0)
  }
  $field = ($tally.GetEnumerator() | Sort-Object Value -Descending | Select-Object -First 1)
  "{0}: surface=#{1:X2}{2:X2}{3:X2} text=#{4:X2}{5:X2}{6:X2} search-field=#{7} x{8} ({9}x{10})" -f $name, $surface.R, $surface.G, $surface.B, $text.R, $text.G, $text.B, $field.Name, $field.Value, $w, $hh
  $bmp.Dispose(); $g.Dispose()
}

try {
  foreach ($want in 'light', 'dark') {
    Set-Content -Path $settings -Value ('{ "theme": "' + $want + '" }') -Encoding utf8
    $exe = Join-Path $PSScriptRoot '..\..\target\release\shortcut-sheet.exe'
    $p = Start-Process -FilePath $exe -ArgumentList '--peek' -PassThru
    Start-Sleep -Seconds 5
    foreach ($h in [W.U]::Of([uint32]$p.Id)) {
      $sb = New-Object System.Text.StringBuilder 256
      [void][W.U]::GetWindowTextW($h, $sb, 256)
      if ($sb.ToString() -like 'shortcut-sheet-overlay*' -and [W.U]::IsWindowVisible($h)) {
        Shoot $h "clone-S01-$want"
      }
    }
    Stop-Process -Id $p.Id -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 400
  }
} finally {
  if ($hadFile) { Copy-Item $backup $settings -Force; "settings.json restored" }
  else { Remove-Item $settings -ErrorAction SilentlyContinue; "settings.json removed (there was none before)" }
}
