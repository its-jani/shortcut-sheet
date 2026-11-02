# Does the search box really take the caret, filter, and give the caret back?
#
# The hook ignores injected input by design, so --peek is the only way to reach S01 from a
# script, and peek mode installs no hook at all, so keystrokes land on the window normally.
#
# Three runs, because there are three ways out of a focused box and they are different code:
#   enter   - type a query, press Enter, and check the combo that fired is the one the *filtered*
#            list put on top, not the one that was there before typing.
#   arrow   - type, press Down, and press Enter: proves the arrows move the selection and that
#            Enter fires the *selected* row rather than always the first one.
#   escape  - press Escape with nothing typed, which must close the panel and hand the caret
#            back without firing anything at all.
# All three then re-read the foreground window: the overlay's whole promise is that you never
# left your app, and "it came back" is only evidence if something was known to have taken it.
#
# WARNING: the enter and arrow runs fire a real combo into whatever window is in front.
# That is the test.
param(
    [string]$App = 'Google Chrome',
    [string]$Only = 'both'          # both | enter | arrow | escape
)
Add-Type -Namespace W -Name Dpi -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
'@
[void][W.Dpi]::SetProcessDpiAwarenessContext([IntPtr](-4))

Add-Type -Namespace W -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int i);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
[DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
[DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, IntPtr extra);
[DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, IntPtr extra);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
public static System.Collections.Generic.List<System.IntPtr> Of(uint wantPid) {
  var list = new System.Collections.Generic.List<System.IntPtr>();
  EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == wantPid) list.Add(h); return true; }, IntPtr.Zero);
  return list;
}
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
[StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
'@
Add-Type -AssemblyName System.Windows.Forms, System.Drawing

function Title([IntPtr]$h) {
  if ($h -eq [IntPtr]::Zero) { return '<none>' }
  $sb = New-Object System.Text.StringBuilder 256
  [void][W.U]::GetWindowTextW($h, $sb, 256)
  $t = $sb.ToString(); if (-not $t) { $t = '<no title>' }
  if ($t.Length -gt 46) { $t = $t.Substring(0, 46) + '...' }
  $t
}

function Type-Key([byte]$vk) {
  [W.U]::keybd_event($vk, 0, 0, [IntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  [W.U]::keybd_event($vk, 0, 2, [IntPtr]::Zero)   # KEYEVENTF_KEYUP
  Start-Sleep -Milliseconds 60
}

function Grab([IntPtr]$h) {
  # PrintWindow, not a screen grab - see the note in peek.ps1 about the white band that was
  # really the page behind coming back through a BitBlt of a GL-composited window.
  $r = New-Object W.U+RECT
  [void][W.U]::GetWindowRect($h, [ref]$r)
  $bmp = New-Object System.Drawing.Bitmap ($r.Right - $r.Left), ($r.Bottom - $r.Top)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $hdc = $g.GetHdc()
  [void][W.U]::PrintWindow($h, $hdc, 2)
  $g.ReleaseHdc($hdc)
  ,@($bmp, $g)
}

function Shoot([IntPtr]$h, [string]$name) {
  $pair = Grab $h
  $out = Join-Path $PSScriptRoot ("..\screens\{0}.png" -f $name)
  $pair[0].Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
  $pair[0].Dispose(); $pair[1].Dispose()
  "captured         : $name.png"
}

$exe = Join-Path $PSScriptRoot '..\..\target\release\shortcut-sheet.exe'
# The lines this probe reads are `debug`, and the app logs `info` unless told otherwise.
$env:RUST_LOG = 'debug'
$log = Get-ChildItem "$env:LOCALAPPDATA\shortcut-sheet\logs" | Sort-Object LastWriteTime | Select-Object -Last 1

foreach ($phase in @('enter', 'arrow', 'escape') | Where-Object { $Only -eq 'both' -or $Only -eq $_ }) {
  ""
  "=== phase: $phase ==="
  $mark = if ($log) { (Get-Content $log.FullName).Count } else { 0 }
  $before = [W.U]::GetForegroundWindow()
  "foreground before: $(Title $before)"

  # Two arguments, and the second one quoted by hand: Start-Process joins -ArgumentList with
  # spaces and adds no quoting of its own, so a bare 'Google Chrome' arrives as two argv
  # entries and the peek path names "Google", quits on purpose, and leaves nothing to click.
  # (Measured: the log says `--peek named an app that is not in the dataset app="Google"`.)
  $p = Start-Process -FilePath $exe -ArgumentList '--peek', "`"$App`"" -PassThru

  # Poll for the window instead of sleeping a fixed 5 s: GL init plus the first frame is 1-6 s
  # on this machine, and a fixed wait reads "no visible overlay" on a run that was fine.
  $h_overlay = [IntPtr]::Zero
  for ($try = 0; $try -lt 20 -and $h_overlay -eq [IntPtr]::Zero; $try++) {
    Start-Sleep -Milliseconds 500
    foreach ($h in [W.U]::Of([uint32]$p.Id)) {
      $sb = New-Object System.Text.StringBuilder 256
      [void][W.U]::GetWindowTextW($h, $sb, 256)
      if ($sb.ToString() -like 'shortcut-sheet-overlay*' -and [W.U]::IsWindowVisible($h)) { $h_overlay = $h }
    }
  }
  if ($h_overlay -eq [IntPtr]::Zero) { "no visible overlay after 10 s"; Stop-Process -Id $p.Id -ErrorAction SilentlyContinue; continue }

  # The panel is centred on whatever was in front when it appeared, and the click below is a
  # coordinate on that panel. If a person is using the machine, the foreground window changes
  # under the probe, the panel is no longer where it was measured, and the click lands on their
  # page instead - which produces every kind of wrong answer except an obvious one. So check
  # that nothing moved, and say so rather than reporting a failure of the app.
  if ((Title ([W.U]::GetForegroundWindow())) -ne (Title $before)) {
    "foreground moved  : $(Title $before) -> $(Title ([W.U]::GetForegroundWindow()))"
    "run discarded     : the desktop is being used; re-run it when it is quiet"
    Stop-Process -Id $p.Id -ErrorAction SilentlyContinue
    continue
  }
  Shoot $h_overlay "search-$phase-before"

  # Where is the search row? Logical math says 12 pad + 32 title = y 44, 30 tall, centre 59,
  # and at 200% that is physical 118 - but the pill is drawn tighter than its layout slot, and
  # a click on its underline is a click on nothing. So measure: scan down the middle column and
  # take the middle of the field. The fill is `card`, which differs from `surface` in both
  # themes, so the band is findable either way.
  $pair = Grab $h_overlay
  $bmp = $pair[0]
  $r = New-Object W.U+RECT
  [void][W.U]::GetWindowRect($h_overlay, [ref]$r)
  $w = $r.Right - $r.Left; $hh = $r.Bottom - $r.Top
  $fx = [int]($w * 0.44)
  $surface = $bmp.GetPixel(60, [int]($hh * 0.82))
  $run = @()
  for ($y = 40; $y -lt [int]($hh * 0.4); $y++) {
    $c = $bmp.GetPixel($fx, $y)
    if ([Math]::Abs($c.R - $surface.R) + [Math]::Abs($c.G - $surface.G) + [Math]::Abs($c.B - $surface.B) -gt 8) { $run += $y }
  }
  # The longest *contiguous* stretch, not first-to-last: the app title's glyphs cross this
  # column a few rows above the field, and spanning them would put the click between the two.
  $best = @(); $cur = @()
  foreach ($y in $run) {
    if ($cur.Count -gt 0 -and $y -eq $cur[-1] + 1) { $cur += $y } else { $cur = @($y) }
    if ($cur.Count -gt $best.Count) { $best = $cur }
  }
  # Where is the search row? The colour scan above used to answer it, and stopped working when
  # the Step 2 measurements were applied: in the overlay the field and the panel are the *same*
  # measured hex (#1F1F1F), so the recessed box is drawn with a rule and a magnifier, not a tint,
  # and there is no band for a pixel scan to find. The tokens are the primary answer now -
  # 12 frame margin + 36 title bar + 2 spacing + 19 (half of the 38 tall search band, minus the
  # 2 px rule) = y 69 logical, scaled by the panel's own device/logical ratio.
  $bmp.Dispose(); $pair[1].Dispose()
  $scale = $w / 360.0
  $cy_logical = 12 + 36 + 2 + 19
  $click_y = [int]($cy_logical * $scale)
  if ($best.Count -ge 6) {
    "band scan         : y $($best[0])..$($best[-1]) (kept as a cross-check, not used)"
  } else {
    "band scan         : nothing to see - field and surface are one measured colour, using tokens"
  }
  $cx = $r.Left + $fx
  $cy = $r.Top + $click_y
  "clicking search   : $cx,$cy = logical ($([int]($fx / $scale)), $cy_logical), scale $scale"
  # Three attempts, and the cursor is re-read before every click. SetCursorPos and the click are
  # two calls with a sleep between them, and on a desktop someone is actually using the pointer
  # moves in that gap - the click then lands on their page, the box never takes focus, and the
  # run reports a broken feature that was never touched. A missed click is retried; a missed
  # feature is what gets reported.
  $focused = $false
  for ($try = 0; $try -lt 3 -and -not $focused; $try++) {
    [void][W.U]::SetCursorPos($cx, $cy)
    Start-Sleep -Milliseconds 120
    $cp = New-Object W.U+POINT
    [void][W.U]::GetCursorPos([ref]$cp)
    if ($cp.X -ne $cx -or $cp.Y -ne $cy) {
      "cursor moved by someone ($($cp.X),$($cp.Y)) - retrying"
      continue
    }
    [W.U]::mouse_event(0x0002, 0, 0, 0, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 60
    [W.U]::mouse_event(0x0004, 0, 0, 0, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 600
    $focused = ([W.U]::GetForegroundWindow() -eq $h_overlay)
  }
  "overlay focused    : $focused (click-to-activate)"
  # Park the pointer off the panel before typing. Hovering a row moves the keyboard selection to
  # it - menu semantics, deliberate - so a cursor left sitting on row 3 turns "press Down, check
  # row 2 is highlighted" into a measurement of where the mouse happened to be. (Measured: the
  # first arrow run reported Ctrl+Shift+N, the third row, because a hand was on the mouse.)
  [void][W.U]::SetCursorPos(4, 4)
  # The style bit is the second opinion. WantFocus clears WS_EX_NOACTIVATE on purpose, so a
  # window that still reads NOACTIVATE=True after the click never took the caret.
  $ex = [W.U]::GetWindowLong($h_overlay, -20)
  "exstyle now        : 0x{0:X8} NOACTIVATE={1}" -f $ex, (($ex -band 0x08000000) -ne 0)
  Shoot $h_overlay "search-$phase-clicked"

  if ($phase -eq 'escape') {
    Type-Key 0x1B
  } elseif (-not $focused) {
    "not typing        : the box never took focus, so keys would go to the app behind"
  } else {
    # Chrome's Everyday collection has several "New ..." commands, so "new" is a query that
    # narrows the list without emptying it.
    foreach ($vk in 0x4E, 0x45, 0x57) { Type-Key $vk }
    Start-Sleep -Milliseconds 800
    Shoot $h_overlay "search-$phase-filtered"
    if ($phase -eq 'arrow') {
      Type-Key 0x28   # Down: the selection has to visibly move before Enter is worth pressing
      Start-Sleep -Milliseconds 500
      Shoot $h_overlay 'search-arrow-moved'
    }
    Type-Key 0x0D
  }
  Start-Sleep -Milliseconds 1500   # let the fired line reach the log before the process goes

  $after = [W.U]::GetForegroundWindow()
  "foreground after   : $(Title $after)"
  "focus restored     : $($after -eq $before)"
  "overlay still shown: $([W.U]::IsWindowVisible($h_overlay))"
  Stop-Process -Id $p.Id -ErrorAction SilentlyContinue

  "--- this run's log (from line $($mark + 1)) ---"
  # The log writer is non_blocking and Stop-Process does not flush it, so the tail of a run can
  # be missing here even though the app did write it. Absence of a line is not proof of absence
  # of the event; the window readings above are the primary evidence and this is the second.
  if ($log) {
    Get-Content $log.FullName | Select-Object -Skip $mark |
      Select-String -Pattern 'fired|takes focus|hands focus|not in the dataset|error|warn' |
      Select-Object -Last 12 | ForEach-Object { $_.Line }
  }
  Start-Sleep -Milliseconds 400
}

# The cursor was moved onto the panel and left there; park it in the corner so a hover row
# does not outlive the run.
[void][W.U]::SetCursorPos(4, 4)
