$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$src = @'
using System;using System.Runtime.InteropServices;
public class K{
 [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
 public struct R{public int L,T,Rr,B;}
}
'@
Add-Type -TypeDefinition $src

$dir = 'd:\shortcut-sheet\replica\screens'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$steps = 'd:\shortcut-sheet\replica\tmp\capture-log.txt'
$log = New-Object System.Collections.Generic.List[string]

function Grab([string]$name) {
  $b = [System.Windows.Forms.SystemInformation]::VirtualScreen
  if (-not $b) { $b = New-Object Drawing.Rectangle(0,0,1920,1080) }
  $bmp = New-Object System.Drawing.Bitmap($b.Width, $b.Height)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
  $path = Join-Path $dir $name
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  return $path
}

Add-Type -AssemblyName System.Windows.Forms
$vkCtrl = 0x11
$before = [K]::GetForegroundWindow()
$log.Add("foreground before: $before")

# baseline: no key held
$log.Add("saved: " + (Grab 'original-00-baseline.png'))

try {
  [K]::keybd_event($vkCtrl, 0, 0, [UIntPtr]::Zero)          # Ctrl down
  Start-Sleep -Milliseconds 2200                             # hold past the trigger delay
  $r = New-Object K+R
  [void][K]::GetWindowRect($before, [ref]$r)
  $log.Add("foreground rect: $($r.L),$($r.T) $(($r.Rr-$r.L))x$(($r.B-$r.T))")
  $log.Add("saved: " + (Grab 'original-01-ctrl-held.png'))
  Start-Sleep -Milliseconds 1500
  $log.Add("saved: " + (Grab 'original-02-ctrl-held-later.png'))
} finally {
  [K]::keybd_event($vkCtrl, 0, 2, [UIntPtr]::Zero)          # Ctrl up (KEYEVENTF_KEYUP)
  $log.Add("ctrl released")
}
Start-Sleep -Milliseconds 600
$log.Add("saved: " + (Grab 'original-03-after-release.png'))
$log | Set-Content -Path $steps -Encoding UTF8
Write-Output "done"
