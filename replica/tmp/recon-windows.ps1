$ErrorActionPreference = 'Stop'
$out = 'd:\shortcut-sheet\replica\tmp\windows.txt'
New-Item -ItemType Directory -Force -Path (Split-Path $out) | Out-Null

$src = @'
using System;using System.Text;using System.Runtime.InteropServices;
public class W{
 [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
 [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
 [DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int i);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 public delegate bool EnumProc(IntPtr h, IntPtr l);
 public struct R{public int L,T,Rr,B;}
 public static string Info(IntPtr h){
   var t=new StringBuilder(512);GetWindowText(h,t,512);
   var c=new StringBuilder(512);GetClassName(h,c,512);
   uint pid;GetWindowThreadProcessId(h,out pid);
   R r;GetWindowRect(h,out r);
   int g=GetWindowLong(h,-16),e=GetWindowLong(h,-20);
   return string.Format("hwnd={0} pid={1} vis={2} class={3} title=[{4}] style=0x{5:X8} ex=0x{6:X8} rect={7},{8} {9}x{10}",
     h,pid,IsWindowVisible(h),c,t,g,e,r.L,r.T,(r.Rr-r.L),(r.B-r.T));
 }
}
'@
Add-Type -TypeDefinition $src -Language CSharp

$lines = New-Object System.Collections.Generic.List[string]
$cb = [W+EnumProc]{ param($h,$l) $script:list.Add($h) | Out-Null; return $true }
$script:list = New-Object System.Collections.Generic.List[IntPtr]
[void][W]::EnumWindows($cb,[IntPtr]::Zero)

$lines.Add("total top-level windows: $($script:list.Count)")
$lines.Add("--- CtrlHelp pid 33484 windows ---")
foreach ($h in $script:list) {
  $i = [W]::Info($h)
  if ($i -match 'pid=33484\b') { $lines.Add($i) }
}
$lines.Add("--- foreground ---")
$lines.Add([W]::Info([W]::GetForegroundWindow()))
$lines | Set-Content -Path $out -Encoding UTF8
Write-Output "wrote $out"
