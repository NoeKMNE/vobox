param(
    [string]$Source = (Join-Path $PSScriptRoot '..\src\assets\app-icon.png')
)

Add-Type -AssemblyName System.Drawing

$root = Split-Path -Parent $PSScriptRoot
$tauriIcons = Join-Path $root 'src-tauri\icons'
$publicDir = Join-Path $root 'public'

$script:src = [System.Drawing.Bitmap]::new($Source)
$script:cache = @{}

function Get-IconBmp([int]$size) {
    if (-not $script:cache.ContainsKey($size)) {
        $bmp = [System.Drawing.Bitmap]::new($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
        $g.DrawImage($script:src, [System.Drawing.Rectangle]::new(0, 0, $size, $size))
        $g.Dispose()
        $script:cache[$size] = $bmp
    }
    return $script:cache[$size]
}

function Save-Png([int]$size, [string]$path) {
    (Get-IconBmp $size).Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
}

function Get-PngBytes([int]$size) {
    $ms = [System.IO.MemoryStream]::new()
    (Get-IconBmp $size).Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $b = $ms.ToArray()
    $ms.Dispose()
    , $b
}

function Write-BeU32($bw, [uint32]$v) {
    $b = [BitConverter]::GetBytes($v)
    $bw.Write($b[3])
    $bw.Write($b[2])
    $bw.Write($b[1])
    $bw.Write($b[0])
}

function Write-Ico([string]$path, [array]$entries) {
    $ms = [System.IO.MemoryStream]::new()
    $bw = [System.IO.BinaryWriter]::new($ms)
    $bw.Write([uint16]0)
    $bw.Write([uint16]1)
    $bw.Write([uint16]$entries.Count)
    $offset = 6 + 16 * $entries.Count
    foreach ($e in $entries) {
        $dim = if ($e.Size -ge 256) { [byte]0 } else { [byte]$e.Size }
        $bw.Write($dim)
        $bw.Write($dim)
        $bw.Write([byte]0)
        $bw.Write([byte]0)
        $bw.Write([uint16]1)
        $bw.Write([uint16]32)
        $bw.Write([uint32]$e.Bytes.Length)
        $bw.Write([uint32]$offset)
        $offset += $e.Bytes.Length
    }
    foreach ($e in $entries) { $bw.Write($e.Bytes) }
    $bytes = $ms.ToArray()
    $bw.Dispose()
    $ms.Dispose()
    [System.IO.File]::WriteAllBytes($path, $bytes)
}

function Write-Icns([string]$path, [array]$chunks) {
    $ms = [System.IO.MemoryStream]::new()
    $bw = [System.IO.BinaryWriter]::new($ms)
    $bw.Write([char[]]'icns')
    $total = 8
    foreach ($c in $chunks) { $total += 8 + $c.Bytes.Length }
    Write-BeU32 $bw ([uint32]$total)
    foreach ($c in $chunks) {
        $bw.Write([char[]]$c.Type)
        Write-BeU32 $bw ([uint32](8 + $c.Bytes.Length))
        $bw.Write($c.Bytes)
    }
    $bytes = $ms.ToArray()
    $bw.Dispose()
    $ms.Dispose()
    [System.IO.File]::WriteAllBytes($path, $bytes)
}

# --- PNG nommés (Tauri + favicon web) ---
Save-Png 32 (Join-Path $tauriIcons '32x32.png')
Save-Png 128 (Join-Path $tauriIcons '128x128.png')
Save-Png 256 (Join-Path $tauriIcons '128x128@2x.png')
Save-Png 512 (Join-Path $tauriIcons 'icon.png')
Save-Png 256 (Join-Path $publicDir 'favicon.png')

# --- Logos Windows (installer / menu démarrer) ---
$squares = @{
    'StoreLogo.png'            = 50
    'Square30x30Logo.png'      = 30
    'Square44x44Logo.png'      = 44
    'Square71x71Logo.png'      = 71
    'Square89x89Logo.png'      = 89
    'Square107x107Logo.png'    = 107
    'Square142x142Logo.png'    = 142
    'Square150x150Logo.png'    = 150
    'Square284x284Logo.png'    = 284
    'Square310x310Logo.png'    = 310
}
foreach ($name in $squares.Keys) {
    Save-Png $squares[$name] (Join-Path $tauriIcons $name)
}

# --- icon.ico (entrées PNG : 16 -> 256) ---
$icoSizes = 16, 24, 32, 48, 64, 128, 256
$icoEntries = foreach ($s in $icoSizes) {
    @{ Size = $s; Bytes = (Get-PngBytes $s) }
}
Write-Ico (Join-Path $tauriIcons 'icon.ico') $icoEntries

# --- icon.icns (macOS, entrées PNG) ---
$icnsChunks = @(
    @{ Type = 'ic11'; Size = 32 },
    @{ Type = 'ic12'; Size = 64 },
    @{ Type = 'ic07'; Size = 128 },
    @{ Type = 'ic08'; Size = 256 },
    @{ Type = 'ic09'; Size = 512 },
    @{ Type = 'ic10'; Size = 1024 }
) | ForEach-Object { @{ Type = $_.Type; Bytes = (Get-PngBytes $_.Size) } }
Write-Icns (Join-Path $tauriIcons 'icon.icns') $icnsChunks

# --- nettoyage ---
foreach ($bmp in $script:cache.Values) { $bmp.Dispose() }
$script:src.Dispose()

Get-ChildItem $tauriIcons | Sort-Object Name | Select-Object Name, Length
Get-Item (Join-Path $publicDir 'favicon.png') | Select-Object Name, Length
