<#
.SYNOPSIS
  Memangkas ext4.vhdx milik WSL agar ruang C: yang terpotong kembali ke disk.

.DESCRIPTION
  Berkas ext4.vhdx Ferrix occupies 71,84 GB on C:, while its contents are
  only 49 GB: the rest is space the guest freed and the host never got
  back. This script stops WSL, compacts the virtual disk, and reports how
  much C: gained.

  It asks for Administrator by itself: when it is not run elevated, it
  re-launches itself with "Run as administrator" and stops.

  Usage (in any PowerShell):
    powershell -ExecutionPolicy Bypass -File .\compact-vhd.ps1

  Options:
    -AlsoWindowsCache   Also clears the Windows Update download cache
                        (C:\Windows\SoftwareDistribution\Download, about 3 GB).

  After it finishes, WSL stays stopped. Start it again with:
    wsl -d archlinux
#>
[CmdletBinding()]
param(
    [switch]$AlsoWindowsCache,
    [switch]$CheckOnly
)

# --- raise ourselves to Administrator -------------------------------------
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    if ($CheckOnly) {
        Write-Host "Butuh hak Administrator untuk Optimize-VHD. Jalankan ulang sebagai Administrator." -ForegroundColor Yellow
        exit 2
    }
    Write-Host "Meminta hak Administrator..." -ForegroundColor Cyan
    $argList = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    if ($AlsoWindowsCache) { $argList += '-AlsoWindowsCache' }
    try {
        Start-Process -FilePath 'powershell.exe' -Verb RunAs -ArgumentList $argList
        Write-Host "Jendela baru terbuka sebagai Administrator. Tunggu sampai selesai." -ForegroundColor Green
    } catch {
        Write-Host "Gagal meminta hak Administrator: $($_.Exception.Message)" -ForegroundColor Red
        Write-Host "Klik kanan PowerShell -> Run as administrator, lalu jalankan ulang skrip ini." -ForegroundColor Yellow
    }
    exit 0
}

function Get-FreeGB {
    $d = Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='C:'"
    [math]::Round($d.FreeSpace / 1GB, 2)
}

Write-Host "Administrator: ya" -ForegroundColor Green
$freeBefore = Get-FreeGB
Write-Host "C: sebelum: $freeBefore GB bebas" -ForegroundColor Yellow

# --- find the virtual disk -------------------------------------------------
$vhdx = $null
$guess = 'C:\Users\jeani\AppData\Local\wsl'
if (Test-Path $guess) {
    $found = Get-ChildItem $guess -Recurse -Filter 'ext4.vhdx' -ErrorAction SilentlyContinue |
             Select-Object -First 1
    if ($found) { $vhdx = $found.FullName }
}
if (-not $vhdx) {
    $reg = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Lxss'
    if (Test-Path $reg) {
        foreach ($k in Get-ChildItem $reg -ErrorAction SilentlyContinue) {
            $base = (Get-ItemProperty $k.PSPath -ErrorAction SilentlyContinue).BasePath
            if ($base) {
                $p = Join-Path $base 'ext4.vhdx'
                if (Test-Path $p) { $vhdx = $p; break }
            }
        }
    }
}
if (-not $vhdx) {
    Write-Host "ext4.vhdx tidak ditemukan" -ForegroundColor Red
    exit 1
}

$sizeBefore = [math]::Round((Get-Item $vhdx).Length / 1GB, 2)
Write-Host "vhdx: $vhdx" -ForegroundColor Cyan
Write-Host "vhdx sebelum: $sizeBefore GB" -ForegroundColor Cyan

if ($CheckOnly) {
    Write-Host "Mode periksa: tidak ada yang diubah." -ForegroundColor Yellow
    exit 0
}

# --- optional: the Windows Update cache ------------------------------------
if ($AlsoWindowsCache) {
    Write-Host "`nMembersihkan cache pembaruan Windows..." -ForegroundColor Cyan
    $wu = 'C:\Windows\SoftwareDistribution\Download'
    if (Test-Path $wu) {
        try {
            Get-ChildItem $wu -Force -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force -ErrorAction Stop
            Write-Host "  cache pembaruan Windows dihapus" -ForegroundColor Green
        } catch {
            Write-Host "  gagal: $($_.Exception.Message)" -ForegroundColor Yellow
        }
    }
    try { Clear-RecycleBin -Force -ErrorAction Stop } catch { }
}

# --- stop WSL ---------------------------------------------------------------
Write-Host "`nMematikan WSL..." -ForegroundColor Cyan
& wsl.exe --shutdown 2>&1 | Out-Null
Start-Sleep -Seconds 20
# Make sure no process still holds the disk.
Get-Process -Name 'vmmem', 'vmmemWSL', 'wsl', 'wslhost' -ErrorAction SilentlyContinue |
    Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 5

# --- compact ----------------------------------------------------------------
Write-Host "Memangkas vhdx..." -ForegroundColor Cyan
$done = $false

if (Get-Command Optimize-VHD -ErrorAction SilentlyContinue) {
    try {
        Optimize-VHD -Path $vhdx -Mode Full -ErrorAction Stop
        $done = $true
        Write-Host "Optimize-VHD selesai." -ForegroundColor Green
    } catch {
        Write-Host "Optimize-VHD gagal: $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

if (-not $done) {
    Write-Host "Modul Hyper-V tidak ada; memakai diskpart." -ForegroundColor Yellow
    $dp = Join-Path $env:TEMP 'ferrix-compact.txt'
    @(
        "select vdisk file=`"$vhdx`"",
        'attach vdisk readonly',
        'compact vdisk',
        'detach vdisk',
        'exit'
    ) | Set-Content -Path $dp -Encoding ASCII
    $out = & diskpart /s $dp 2>&1 | Out-String
    if ($out -match 'successfully compacted|berhasil dikompresikan|Virtual Disk Compact') {
        $done = $true
        Write-Host "diskpart selesai." -ForegroundColor Green
    } else {
        Write-Host "diskpart tidak melaporkan keberhasilan:" -ForegroundColor Yellow
        Write-Host $out
    }
}

# --- report -----------------------------------------------------------------
$sizeAfter = [math]::Round((Get-Item $vhdx).Length / 1GB, 2)
$freeAfter = Get-FreeGB
Write-Host "`n=== hasil ===" -ForegroundColor Cyan
Write-Host ("vhdx: {0} GB -> {1} GB  (hemat {2} GB)" -f $sizeBefore, $sizeAfter, [math]::Round($sizeBefore - $sizeAfter, 2))
Write-Host ("C:: {0} GB -> {1} GB bebas  (tambah {2} GB)" -f $freeBefore, $freeAfter, [math]::Round($freeAfter - $freeBefore, 2))
Write-Host "WSL masih berhenti. Mulai lagi dengan: wsl -d archlinux" -ForegroundColor Green
if (-not $done) { exit 3 }
