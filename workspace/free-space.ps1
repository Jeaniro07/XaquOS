<#
.SYNOPSIS
  Membebaskan ruang disk di C: untuk build ISO Ferrix.

.DESCRIPTION
  Tiga bagian, masing-masing aman dan bisa dipakai terpisah:

    1. Sampah Windows  (tanpa admin)   : temp, cache pembaruan Windows, cache
                                          peramban, cache npm/pip, laporan galat,
                                          Recycle Bin.
    2. Sampah di WSL   (tanpa admin)   : artefak tes yang bisa dibangkitkan
                                          ulang, journal, /tmp, dan (opsional)
                                          folder target/debug yang besar.
    3. Pemangkasan VHDX (PERLU admin)   : wsl --shutdown lalu Optimize-VHD, yang
                                          mengembalikan ruang terpotong di
                                          ext4.vhdx ke C:.

  Pemakaian:
    # bagikan 1 + 2 saja
    powershell -ExecutionPolicy Bypass -File .\free-space.ps1

    # tambah bagian 3 (jalankan dari PowerShell sebagai Administrator)
    powershell -ExecutionPolicy Bypass -File .\free-space.ps1 -Compact

    # tambah hapus target/debug (14 GB, tapi check harus membangun ulang)
    powershell -ExecutionPolicy Bypass -File .\free-space.ps1 -Deep

.PARAMETER Compact
  Jalankan bagian 3: matikan WSL lalu pangkas ext4.vhdx. Butuh hak Administrator.

.PARAMETER Deep
  Hapus target/debug di dalam WSL (14 GB) dan mirror komponen yang besar
  (wine). Semuanya harus dibangun ulang oleh build berikutnya.

.PARAMETER WhatIf
  Hanya tampilkan yang akan dihapus, tanpa menghapus apa pun.
#>
[CmdletBinding()]
param(
    [switch]$Compact,
    [switch]$Deep,
    [switch]$WhatIf
)

$ErrorActionPreference = 'Continue'

function Get-FreeGB {
    $d = Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='C:'"
    [math]::Round($d.FreeSpace / 1GB, 2)
}

function Write-Step($text) { Write-Host "`n=== $text ===" -ForegroundColor Cyan }
function Write-Info($text) { Write-Host "    $text" }

function Remove-Tree {
    param([string]$Path, [string]$Label)
    if (-not (Test-Path $Path)) { return 0 }
    $bytes = 0
    try {
        $bytes = (Get-ChildItem $Path -Recurse -File -Force -ErrorAction SilentlyContinue |
                  Measure-Object Length -Sum).Sum
    } catch { $bytes = 0 }
    $mb = [math]::Round($bytes / 1MB, 0)
    if ($mb -lt 1) { return 0 }
    if ($WhatIf) {
        Write-Info ("{0,8:N0} MB  {1}  (akan dihapus)" -f $mb, $Label)
        return 0
    }
    try {
        Remove-Item $Path -Recurse -Force -ErrorAction Stop
        Write-Info ("{0,8:N0} MB  {1}  (dihapus)" -f $mb, $Label) -ForegroundColor Green
        return $mb
    } catch {
        Write-Info ("{0,8:N0} MB  {1}  (GAGAL: {2})" -f $mb, $Label, $_.Exception.Message) -ForegroundColor Yellow
        if ($Label -eq 'cache pembaruan Windows') {
            Write-Info "           -> jalankan ulang dari PowerShell Administrator" -ForegroundColor Yellow
        }
        return 0
    }
}

$before = Get-FreeGB
Write-Host "C: sebelum: $before GB bebas" -ForegroundColor Yellow

# ---------------------------------------------------------------- 1. Windows
Write-Step "1. Sampah Windows"

$tempOld = Join-Path $env:LOCALAPPDATA 'Temp'
if (Test-Path $tempOld) {
    $cut = (Get-Date).AddHours(-2)
    $items = Get-ChildItem $tempOld -Force -ErrorAction SilentlyContinue |
             Where-Object { $_.LastWriteTime -lt $cut }
    $bytes = 0
    foreach ($i in $items) {
        if ($i.PSIsContainer) {
            $bytes += (Get-ChildItem $i.FullName -Recurse -File -Force -ErrorAction SilentlyContinue |
                       Measure-Object Length -Sum).Sum
        } else { $bytes += $i.Length }
    }
    $mb = [math]::Round($bytes / 1MB, 0)
    if (-not $WhatIf) {
        foreach ($i in $items) {
            try { Remove-Item $i.FullName -Recurse -Force -ErrorAction Stop } catch { }
        }
    }
    Write-Info ("{0,8:N0} MB  Temp (yang lebih tua dari 1 hari)" -f $mb)
}

$targets = @(
    @{ p = 'C:\Windows\SoftwareDistribution\Download'; l = 'cache pembaruan Windows' },
    @{ p = "$env:LOCALAPPDATA\Microsoft\Edge\User Data\Default\Cache"; l = 'cache Edge' },
    @{ p = "$env:LOCALAPPDATA\Microsoft\Edge\User Data\Default\Code Cache"; l = 'code cache Edge' },
    @{ p = "$env:LOCALAPPDATA\Google\Chrome\User Data\Default\Cache"; l = 'cache Chrome' },
    @{ p = "$env:LOCALAPPDATA\npm-cache"; l = 'cache npm' },
    @{ p = "$env:APPDATA\npm-cache"; l = 'cache npm ( roaming)' },
    @{ p = "$env:LOCALAPPDATA\pip\Cache"; l = 'cache pip' },
    @{ p = "$env:ProgramData\Microsoft\Windows\WER"; l = 'laporan galat Windows' },
    @{ p = "$env:LOCALAPPDATA\Microsoft\Windows\WER"; l = 'laporan galat Windows (pengguna)' },
    @{ p = "$env:LOCALAPPDATA\D3DSCache"; l = 'cache shader DirectX' },
    @{ p = 'C:\ProgramData\Microsoft\Windows\DeliveryOptimization\Cache'; l = 'cache Delivery Optimization' },
    @{ p = "$env:LOCALAPPDATA\CrashDumps"; l = 'dump galat aplikasi' },
    @{ p = 'C:\Windows\Minidump'; l = 'minidump Windows' },
    @{ p = 'C:\Windows\LiveKernelReports'; l = 'laporan kernel' },
    @{ p = "$env:USERPROFILE\Downloads\Ferrix"; l = 'unduhan Ferrix' },
    @{ p = "$env:LOCALAPPDATA\Temp\DiagOutputDir"; l = 'keluaran diagnostik Windows' }
)
foreach ($t in $targets) { Remove-Tree -Path $t.p -Label $t.l | Out-Null }

if (-not $WhatIf) {
    try {
        Clear-RecycleBin -Force -ErrorAction Stop
        Write-Info "Recycle Bin dikosongkan" -ForegroundColor Green
    } catch { }
}

# ------------------------------------------------------------------ 2. WSL
Write-Step "2. Sampah di dalam WSL (Arch)"

$wslScript = @'
set -u
before=$(df -B1 --output=avail / | tail -1)

# Artefak tes yang bisa dibangkitkan ulang.
rm -rf /root/ferrix/target/xtask-native-test 2>/dev/null
rm -rf /root/ferrix/target/doc 2>/dev/null
rm -rf /tmp/nd-check /tmp/nd-base 2>/dev/null
rm -rf /root/.cache/pip /root/.cache/uv 2>/dev/null

# Arsip crate yang sudah dibongkar ke registry/src: unduh ulang bila perlu.
rm -rf /root/.cargo/registry/cache 2>/dev/null

# Toolchain yang tidak dipakai: repo mengikat 1.99.1 dan langkah Ferrocene
# memakai 1.95.0. Nightly hanya dipakai tools/common/gen/host-coverage.py
# (ukuran cakupan, bukan bagian check) dan rustup mengunduhnya lagi bila
# diperlukan.
rm -rf /root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu 2>/dev/null
rm -rf /root/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu 2>/dev/null

# Worktree yang cabangnya sudah digabung ke cabang integrasi; cabangnya
# sendiri tetap ada.
cd /root/ferrix 2>/dev/null || exit 0
for wt in /root/ferrix-a /root/ferrix-b /root/ferrix-c /root/ferrix-fix; do
  [ -d "$wt" ] || continue
  branch=$(git -C "$wt" rev-parse --abbrev-ref HEAD 2>/dev/null)
  [ -n "$branch" ] || continue
  if git merge-base --is-ancestor "$branch" HEAD 2>/dev/null; then
    git worktree remove --force "$wt" 2>/dev/null && echo "worktree $wt ($branch) dibuang: cabangnya sudah digabung"
  else
    echo "worktree $wt ($branch) DIPERTAHANKAN: cabangnya belum digabung"
  fi
done
git worktree prune 2>/dev/null

journalctl --vacuum-size=50M >/dev/null 2>&1
if [ "${FERRIX_DEEP:-0}" = "1" ]; then
  rm -rf /root/ferrix/target/debug 2>/dev/null
  rm -rf /root/.local/share/ferrix/wine 2>/dev/null
fi
sync
fstrim -a >/dev/null 2>&1
after=$(df -B1 --output=avail / | tail -1)
echo "WSL_SGAK_BEFORE=$before"
echo "WSL_SGAK_AFTER=$after"
'@

$wslTmp = Join-Path $env:TEMP 'ferrix-free-space.sh'
$utf8 = New-Object System.Text.UTF8Encoding($false)
[IO.File]::WriteAllText($wslTmp, ($wslScript -replace "`r`n", "`n"), $utf8)
$wslTmpWin = (Join-Path $env:TEMP 'ferrix-free-space.sh') -replace '\\', '/'
$wslTmpWin = '/mnt/' + $wslTmpWin.Substring(0, 1).ToLower() + $wslTmpWin.Substring(2)
$env:FERRIX_DEEP = if ($Deep) { '1' } else { '0' }

if (-not $WhatIf) {
    $out = & wsl.exe -d archlinux -- bash -lc "FERRIX_DEEP=$($env:FERRIX_DEEP) bash $wslTmpWin" 2>&1
    $out | ForEach-Object { Write-Info $_ }
} else {
    Write-Info "akan dijalankan: rm target/xtask-native-test, target/doc, /tmp/nd-*, journal vacuum, fstrim"
    if ($Deep) { Write-Info "  (+ mode Deep: target/debug, mirror wine)" }
}

# ------------------------------------------------------- 3. pangkas VHDX
if ($Compact) {
    Write-Step "3. Memangkas ext4.vhdx (butuh WSL berhenti)"
    $vhdx = 'C:\Users\jeani\AppData\Local\wsl\{c37ddf98-4c8e-47c4-95f4-f87605e879c4}\ext4.vhdx'
    if (-not (Test-Path $vhdx)) {
        $vhdx = (Get-ChildItem "$env:LOCALAPPDATA\wsl" -Recurse -Filter 'ext4.vhdx' -ErrorAction SilentlyContinue |
                 Select-Object -First 1).FullName
    }
    if (-not $vhdx) { Write-Info "ext4.vhdx tidak ditemukan" -ForegroundColor Red }
    else {
        $sizeBefore = [math]::Round((Get-Item $vhdx).Length / 1GB, 2)
        Write-Info "vhdx sebelum: $sizeBefore GB"
        Write-Info "matikan WSL..."
        & wsl.exe --shutdown 2>&1 | Out-Null
        Start-Sleep -Seconds 20
        $ok = $false
        if (Get-Command Optimize-VHD -ErrorAction SilentlyContinue) {
            try {
                Optimize-VHD -Path $vhdx -Mode Full -ErrorAction Stop
                $ok = $true
            } catch { Write-Info "Optimize-VHD gagal: $($_.Exception.Message)" -ForegroundColor Yellow }
        } else {
            Write-Info "modul Hyper-V tidak ada; memakai diskpart" -ForegroundColor Yellow
            $dp = Join-Path $env:TEMP 'compact.txt'
            "select vdisk file=`"$vhdx`"`nattach vdisk readonly`ncompact vdisk`ndetach vdisk`nexit" |
                Set-Content -Path $dp -Encoding ASCII
            & diskpart /s $dp 2>&1 | Out-Null
            $ok = $true
        }
        if ($ok) {
            $sizeAfter = [math]::Round((Get-Item $vhdx).Length / 1GB, 2)
            Write-Info ("vhdx sesudah: {0} GB  (hemat {1} GB)" -f $sizeAfter, [math]::Round($sizeBefore - $sizeAfter, 2)) -ForegroundColor Green
        }
    }
} else {
    Write-Step "3. Pemangkasan VHDX dilewati"
    Write-Info "jalankan ulang dengan -Compact dari PowerShell Administrator untuk memangkas ext4.vhdx"
}

$after = Get-FreeGB
Write-Host "`nC: sesudah: $after GB bebas  (tambah $([math]::Round($after - $before, 2)) GB)" -ForegroundColor Yellow
Write-Host "Selesai. WSL bisa dijalankan lagi dengan: wsl -d archlinux" -ForegroundColor Cyan
