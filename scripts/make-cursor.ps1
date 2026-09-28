Add-Type -AssemblyName System.Drawing

$w = 32
$h = 32
$bmp = [System.Drawing.Bitmap]::new($w, $h, [System.Drawing.Imaging.PixelFormat]::Format32bppPArgb)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

# Flèche classique (pointe en haut à gauche), échelle réduite + coins arrondis
$scale = 1.1
$off = 3.0
$raw = @(
    @(0, 0),
    @(0, 17.5),
    @(4.4, 13.5),
    @(7, 19.3),
    @(9.5, 18.2),
    @(6.9, 12.6),
    @(11.5, 12.6)
)
$pts = foreach ($p in $raw) {
    [System.Drawing.PointF]::new([double]($p[0] * $scale + $off), [double]($p[1] * $scale + $off))
}

$path = [System.Drawing.Drawing2D.GraphicsPath]::new()
$path.AddPolygon($pts)

# Couleur par défaut = teinte analogue du violet accent du thème clair (hsl(289, 90%, 40%))
$fillColor = [System.Drawing.Color]::FromArgb(255, 160, 10, 194)
$outlineColor = [System.Drawing.Color]::FromArgb(255, 0, 0, 0)

$brushFill = [System.Drawing.SolidBrush]::new($fillColor)
$brushOut = [System.Drawing.SolidBrush]::new($outlineColor)

# Passe 1 : contour gonflé de 1.5 px avec joints arrondis
$penOut = [System.Drawing.Pen]::new($outlineColor, [float]3.0)
$penOut.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
# Passe 2 : remplissage gonflé de 0.45 px (anneau de contour ~1 px)
$penFill = [System.Drawing.Pen]::new($fillColor, [float]0.9)
$penFill.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round

$g.DrawPath($penOut, $path)
$g.FillPath($brushOut, $path)
$g.DrawPath($penFill, $path)
$g.FillPath($brushFill, $path)

$out = Join-Path $PSScriptRoot '..\public\cursors\arrow.png'
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)

$g.Dispose()
$bmp.Dispose()
$brushFill.Dispose()
$brushOut.Dispose()
$penOut.Dispose()
$penFill.Dispose()

Get-Item $out | Select-Object FullName, Length
