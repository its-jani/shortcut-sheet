# Crop the overlay out of a full-screen capture so the panel can be inspected at 1:1.
param(
    [string]$Source = (Join-Path $PSScriptRoot 'fullscreen.png'),
    [string]$Out = (Join-Path $PSScriptRoot 'panel.png'),
    [int]$X = 208, [int]$Y = 31, [int]$W = 384, [int]$H = 464
)
Add-Type -AssemblyName System.Drawing
$bmp = [System.Drawing.Bitmap]::FromFile($Source)
$rect = New-Object System.Drawing.Rectangle $X, $Y, $W, $H
$crop = $bmp.Clone($rect, $bmp.PixelFormat)
$crop.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
"source : $Source ($($bmp.Width) x $($bmp.Height))"
"crop   : $Out ($W x $H at $X,$Y)"
$crop.Dispose(); $bmp.Dispose()
