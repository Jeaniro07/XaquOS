"""Boot LordOS live ISO in QEMU (WHPX) with a serial console and measure boot time and idle RAM
(spec 2026-11-11 §12.1: 4 GB, 4 cores). Set LORDOS_DIR to the folder holding the verified ISO,
vmlinuz and initrd.img; run with: python lordos.py [whpx|tcg]"""
import socket, subprocess, sys, time, re, os

sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # the console's cp1252 cannot print systemd's glyphs

D = os.environ.get("LORDOS_DIR", os.path.join(os.environ.get("TEMP", "."), "claude",
                   "7e720ee6bd4f3c8d", "9154315e-962f-4f88-b757-d959acdeeb77", "scratchpad", "lordos-iso"))
QEMU = r"C:\Program Files\qemu\qemu-system-x86_64.exe"
PORT = 4555
ACCEL = sys.argv[1] if len(sys.argv) > 1 else "whpx"
CMDLINE = ("boot=live components quiet loglevel=3 splash vt.global_cursor_default=0 "
           "lordos.lang=en console=tty0 console=ttyS0,115200")
args = [QEMU, "-accel", ACCEL, "-machine", "q35", "-m", "4096", "-smp", "4",
        "-cdrom", os.path.join(D, "lordos-live-0.8.8-amd64.iso"),
        "-kernel", os.path.join(D, "vmlinuz"), "-initrd", os.path.join(D, "initrd.img"),
        "-append", CMDLINE, "-vga", "std", "-display", "none",
        "-netdev", "user,id=n0", "-device", "virtio-net-pci,netdev=n0",
        "-serial", f"tcp:127.0.0.1:{PORT},server=on,wait=on"]
log = open(os.path.join(D, f"serial-{ACCEL}.log"), "w", encoding="utf-8", errors="replace")
q = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT)
t0 = time.time()
time.sleep(1)
s = socket.create_connection(("127.0.0.1", PORT))
s.settimeout(1)
buf = ""


def read_until(pat, timeout):
    global buf
    end = time.time() + timeout
    while time.time() < end:
        try:
            data = s.recv(65536).decode("utf-8", "replace")
        except socket.timeout:
            data = ""
        if data:
            buf += data
            log.write(data); log.flush()
        if re.search(pat, buf):
            return True
        if q.poll() is not None:
            return False
    return False


def send(line):
    s.sendall((line + "\n").encode())


ok = read_until(r"login:", 600)
t_login = time.time() - t0
print(f"serial login prompt after {t_login:.1f}s (wall, includes QEMU start): {ok}")
if not ok:
    q.kill(); sys.exit(1)
user = re.findall(r"\n(\S+) login:", buf)
print("prompt line:", user[-1:] if user else "?")
send("root")
if read_until(r"[Pp]assword:", 5):
    send("")
time.sleep(3)
if not re.search(r"[#$] $", buf[-200:]):
    for u in ("lordos", "user"):
        send(u)
        if read_until(r"[Pp]assword:", 5):
            send("live" if u == "user" else "")
        time.sleep(3)
        if re.search(r"[#$] $", buf[-200:]):
            break
print("waiting for graphical session, then 65 s idle")
send("systemctl is-system-running --wait; systemctl list-units --type=service --state=running --no-legend | wc -l")
read_until(r"\n\d+\r?\n", 300)
time.sleep(65)
mark = len(buf)
for c in ["echo ===MEASURE", "systemd-analyze", "systemd-analyze critical-chain graphical.target | head -5",
          "free -m", "grep -E 'MemTotal|MemAvailable' /proc/meminfo", "vmstat 1 3",
          "ps -eo rss,comm --sort=-rss | head -15", "echo ===END"]:
    send(c)
read_until(r"===END\r?\n.*===END", 60)
read_until(r"zzz_never", 3)
print(buf[mark:])
q.kill()
