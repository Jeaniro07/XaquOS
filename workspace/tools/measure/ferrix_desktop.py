"""Ferrix's desktop under KVM in WSL, measured as spec §12.1 says: QEMU start
-> dock drawn, -> graphical.target active; after 65 s idle, MemTotal -
MemAvailable and each process's RSS, read from the serial console.
Run inside WSL: python3 ferrix_desktop.py [~/ferrix]"""
import os, re, subprocess, sys, threading, time

root = os.path.expanduser(sys.argv[1] if len(sys.argv) > 1 else "~/ferrix")
env = dict(os.environ, FERRIX_OVMF_CODE="/usr/share/edk2/x64/OVMF_CODE.4m.fd",
           FERRIX_OVMF_VARS="/usr/share/edk2/x64/OVMF_VARS.4m.fd")
cmd = ["cargo", "xtask", "run-compositor", "--arch", "x86_64", "--vnc", ":0", "--no-gl",
       "--smp", "4", "--memory", "4096", "--accel", "kvm"]
log = open(os.path.expanduser("~/ferrix-measure.log"), "w")
p = subprocess.Popen(cmd, cwd=root, env=env, stdin=subprocess.PIPE,
                     stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True)
lines = []


def pump():
    for raw in iter(p.stdout.readline, b""):
        s = raw.decode("utf-8", "replace").rstrip("\n")
        lines.append((time.time(), s))
        log.write(f"{time.time():.2f} {s}\n")
        log.flush()


threading.Thread(target=pump, daemon=True).start()


def wait_for(pattern, timeout, after=0.0):
    end = time.time() + timeout
    while time.time() < end:
        for t, s in list(lines):
            if t >= after and re.search(pattern, s):
                return t
        if p.poll() is not None:
            return None
        time.sleep(0.2)
    return None


def send(line):
    p.stdin.write((line + "\n").encode())
    p.stdin.flush()


def since(t, t0):
    return "not seen" if t is None or t0 is None else f"{t - t0:.2f} s"


try:
    t0 = wait_for(r"booting \(quit with", 1800)
    dock = wait_for(r"the dock is up", 300, t0 or 0)
    graphical = wait_for(r"graphical\.target.*(active|reached)", 300, t0 or 0)
    print(f"QEMU start -> dock drawn: {since(dock, t0)}", flush=True)
    print(f"QEMU start -> graphical.target: {since(graphical, t0)}", flush=True)
    # A rebuilt image is installed onto the root volume first (about 4 s): such a boot is not idle-boot speed.
    for _, s in list(lines):
        if "root     / " in s:
            print(s.strip(), flush=True)
    time.sleep(65)
    mark = len(lines)
    send("echo ===MEASURE")
    send("while read k v u; do case $k in MemTotal:|MemAvailable:) echo $k $v $u;; esac; done < /proc/meminfo")
    send("for f in /proc/[0-9]*/status; do n=; while read k v r; do case $k in Name:) n=$v;; VmRSS:) echo RSS $v $n;; esac; done < $f; done")
    send("echo ===END")
    wait_for(r"^===END", 60, lines[mark - 1][0] if mark else 0)
    time.sleep(3)
    mem = {}
    for _, s in lines[mark:]:
        m = re.match(r"(MemTotal|MemAvailable): (\d+)", s)
        if m:
            mem[m.group(1)] = int(m.group(2))
        elif s.startswith("RSS "):
            print(s)
    if len(mem) == 2:
        print(f"used (MemTotal - MemAvailable): {(mem['MemTotal'] - mem['MemAvailable']) / 1024:.0f} MiB")
finally:
    # The whole group: cargo, xtask and QEMU, so no QEMU is left holding the image.
    os.killpg(p.pid, 15)
