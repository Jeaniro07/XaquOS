#!/usr/bin/env python3
"""Wire the two new crates into the workspace: the root manifest and the
xtask's list of native programs."""

import sys

ROOT = "/root/ferrix"


def patch(path, anchor, addition, before=False):
    with open(path, "r", encoding="utf-8") as handle:
        text = handle.read()
    if addition.strip() in text:
        print("already patched:", path)
        return
    if anchor not in text:
        print("ANCHOR NOT FOUND in", path, ":", anchor)
        sys.exit(1)
    if before:
        text = text.replace(anchor, addition + anchor, 1)
    else:
        text = text.replace(anchor, anchor + addition, 1)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text)
    print("patched:", path)


root = ROOT + "/Cargo.toml"

patch(
    root,
    '    "src/lib/drivers/input/i2c-hid",\n',
    '    "src/lib/drivers/i2c/lpss-i2c",\n',
)

patch(
    root,
    '    "src/user/system/native/drivers/input/virtio-input",\n',
    '    "src/user/system/native/drivers/input/i2c-hid",\n',
)

patch(
    root,
    'xaqu-i2c-hid = { path = "src/lib/drivers/input/i2c-hid" }\n',
    'xaqu-lpss-i2c = { path = "src/lib/drivers/i2c/lpss-i2c" }\n',
)

patch(
    root,
    'xaqu-hda-driver = { path = "src/user/system/native/drivers/sound/hda" }\n',
    'xaqu-i2c-hid-driver = { path = "src/user/system/native/drivers/input/i2c-hid" }\n',
)

native = ROOT + "/tools/common/xtask/src/native.rs"

patch(
    native,
    """    Program {
        package: "xaqu-input",
        binary: "input",
        directory: DRIVERS,
    },
""",
    """    // The I2C-HID touchpad driver: a touchpad on an Intel Serial I/O (LPSS)
    // I2C controller, polled, served to the input core.
    Program {
        package: "xaqu-i2c-hid-driver",
        binary: "i2c-hid",
        directory: DRIVERS,
    },
""",
)
