# Generates Marq's app icon (PNG sizes + multi-size .ico) and a demo image, using GDI+.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$iconsDir = Join-Path $PSScriptRoot '..\src-tauri\icons' | Resolve-Path
$assetsDir = Join-Path $PSScriptRoot '..\assets' | Resolve-Path

function New-MarqBitmap([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    $g.TextRenderingHint = 'AntiAliasGridFit'
    $g.PixelOffsetMode = 'HighQuality'

    # Rounded-square background with a blue gradient
    $margin = [Math]::Max(1, [int]($size * 0.03))
    $inner = $size - (2 * $margin)
    $rect = New-Object System.Drawing.Rectangle($margin, $margin, $inner, $inner)
    $radius = [Math]::Max(2, [int]($size * 0.22))
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $radius * 2
    $path.AddArc($rect.X, $rect.Y, $d, $d, 180, 90)
    $path.AddArc($rect.Right - $d, $rect.Y, $d, $d, 270, 90)
    $path.AddArc($rect.Right - $d, $rect.Bottom - $d, $d, $d, 0, 90)
    $path.AddArc($rect.X, $rect.Bottom - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    $c1 = [System.Drawing.Color]::FromArgb(255, 68, 130, 245)
    $c2 = [System.Drawing.Color]::FromArgb(255, 18, 64, 158)
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect, $c1, $c2, 55)
    $g.FillPath($brush, $path)

    $white = [System.Drawing.Brushes]::White
    if ($size -ge 32) {
        # "M" left-of-center + a down arrow on the right (classic markdown mark)
        $fontSize = [float]($size * 0.52)
        $font = New-Object System.Drawing.Font('Segoe UI', $fontSize, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
        $sf = New-Object System.Drawing.StringFormat
        $sf.Alignment = 'Center'; $sf.LineAlignment = 'Center'
        $mRect = New-Object System.Drawing.RectangleF(0, [float]($size * 0.02), [float]($size * 0.68), [float]$size)
        $g.DrawString('M', $font, $white, $mRect, $sf)
        # Down arrow: shaft + triangle
        $cx = [float]($size * 0.745)
        $shaftW = [float]($size * 0.075)
        $topY = [float]($size * 0.30)
        $headY = [float]($size * 0.53)
        $tipY = [float]($size * 0.70)
        $headHalf = [float]($size * 0.135)
        $g.FillRectangle($white, $cx - $shaftW / 2, $topY, $shaftW, $headY - $topY + 1)
        $tri = @(
            (New-Object System.Drawing.PointF(($cx - $headHalf), $headY)),
            (New-Object System.Drawing.PointF(($cx + $headHalf), $headY)),
            (New-Object System.Drawing.PointF($cx, $tipY))
        )
        $g.FillPolygon($white, $tri)
    } else {
        # Tiny sizes: just a centered M
        $fontSize = [float]($size * 0.72)
        $font = New-Object System.Drawing.Font('Segoe UI', $fontSize, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
        $sf = New-Object System.Drawing.StringFormat
        $sf.Alignment = 'Center'; $sf.LineAlignment = 'Center'
        $r = New-Object System.Drawing.RectangleF(0, 0, [float]$size, [float]$size)
        $g.DrawString('M', $font, $white, $r, $sf)
    }
    $g.Dispose()
    return $bmp
}

function Get-PngBytes([System.Drawing.Bitmap]$bmp) {
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    return $ms.ToArray()
}

# PNG files Tauri wants
foreach ($s in 32, 128) {
    $bmp = New-MarqBitmap $s
    $bmp.Save("$iconsDir\${s}x${s}.png", [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

# Multi-size ICO (PNG-compressed entries)
$sizes = 256, 64, 48, 32, 16
$blobs = @()
foreach ($s in $sizes) {
    $bmp = New-MarqBitmap $s
    $blobs += , ([byte[]](Get-PngBytes $bmp))
    $bmp.Dispose()
}
$fs = [System.IO.File]::Create("$iconsDir\icon.ico")
$bw = New-Object System.IO.BinaryWriter($fs)
$bw.Write([uint16]0); $bw.Write([uint16]1); $bw.Write([uint16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $s = $sizes[$i]
    $bw.Write([byte]($(if ($s -ge 256) { 0 } else { $s })))  # width
    $bw.Write([byte]($(if ($s -ge 256) { 0 } else { $s })))  # height
    $bw.Write([byte]0); $bw.Write([byte]0)                    # colors, reserved
    $bw.Write([uint16]1); $bw.Write([uint16]32)               # planes, bpp
    $bw.Write([uint32]$blobs[$i].Length)
    $bw.Write([uint32]$offset)
    $offset += $blobs[$i].Length
}
foreach ($b in $blobs) { $bw.Write([byte[]]$b) }
$bw.Close(); $fs.Close()

# Demo image for the showcase document
$bmp = New-Object System.Drawing.Bitmap(640, 320)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'; $g.TextRenderingHint = 'AntiAliasGridFit'
$rect = New-Object System.Drawing.Rectangle(0, 0, 640, 320)
$brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect, [System.Drawing.Color]::FromArgb(255, 88, 101, 242), [System.Drawing.Color]::FromArgb(255, 235, 69, 158), 35)
$g.FillRectangle($brush, $rect)
$font = New-Object System.Drawing.Font('Segoe UI', 28, [System.Drawing.FontStyle]::Bold)
$sf = New-Object System.Drawing.StringFormat; $sf.Alignment = 'Center'; $sf.LineAlignment = 'Center'
$g.DrawString("Local image - loaded by Marq", $font, [System.Drawing.Brushes]::White, (New-Object System.Drawing.RectangleF(0, 0, 640, 320)), $sf)
$g.Dispose()
$bmp.Save("$assetsDir\demo.png", [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

Write-Host "Icons written to $iconsDir"
Get-ChildItem $iconsDir, $assetsDir | Format-Table Name, Length