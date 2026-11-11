"""Yoru's light-glass wallpaper: a 1920x1080 diagonal gradient, blue to lilac
to sand, Ferrix's own. Python's standard library only, so the PNG can be made
again anywhere: python3 make-light-glass.py yoru-light-glass.png"""
import struct, sys, zlib

W, H = 1920, 1080
STOPS = [(0.0, (0xA9, 0xC6, 0xF5)), (0.55, (0xE8, 0xD5, 0xF2)), (1.0, (0xF6, 0xE3, 0xC9))]


def colour(t):
    for (t0, c0), (t1, c1) in zip(STOPS, STOPS[1:]):
        if t <= t1:
            f = (t - t0) / (t1 - t0)
            return tuple(round(a + (b - a) * f) for a, b in zip(c0, c1))
    return STOPS[-1][1]


def chunk(kind, data):
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))


def main(path):
    rows = bytearray()
    span = 0.94 * W + 0.34 * H  # the 160-degree axis, as the mock-up drew it
    for y in range(H):
        raw = bytearray()
        for x in range(W):
            raw += bytes(colour((0.94 * x + 0.34 * y) / span))
        rows.append(1)  # PNG filter "Sub": a gradient's neighbours differ by 0 or 1
        rows += bytes(raw[i] if i < 3 else (raw[i] - raw[i - 3]) & 0xFF for i in range(len(raw)))
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(rows), 9))
    png += chunk(b"IEND", b"")
    with open(path, "wb") as out:
        out.write(png)


if __name__ == "__main__":
    main(sys.argv[1])
