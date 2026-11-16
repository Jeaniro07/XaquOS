//! A driver that dies at every start: `cargo xtask test-restart --boot
//! crashloop`'s, carried under the name `intel` on the QEMU stand-in for
//! ADL-N's GPU, so that devmgr starts it as the box's Intel driver was
//! started on 2026-10-30, when that driver died at every start
//! (`docs/DEVMGR.md` §4, the crash loop's escalation).
//!
//! It reads START as an engine's -- one handle, the device -- pins one page
//! of memory to the device, says so with the monotonic time in
//! milliseconds -- the gate reads the delays between starts from it -- and
//! exits [`DIED`] with the pin still held, so that every death leaves a
//! pin for the kernel's quarantine as a real driver's death at its start
//! would. The line is the gate's proof
//! that each start pinned: a pin refused past twice the device's budget
//! (`QUARANTINE_FULL`) would say `refused` instead.
//!
//! Only that gate's image carries it: xtask puts it under `intel` for the
//! one boot and nowhere else (`tools/common/xtask/src/restart/crashloop.rs`).

#![no_std]
#![no_main]

use xaqu_blkring::control::{Message as StartMessage, START_BYTES};
use xaqu_native_abi::handle::Handle;
use xaqu_native_abi::signals::Signals;
use xaqu_rt::native::device::Device;
use xaqu_rt::native::handle::{Deadline, Object, OwnedHandle};
use xaqu_rt::native::pin::PinAccess;
use xaqu_rt::native::vmo;
use xaqu_rt::{Bootstrap, Kernel};

xaqu_rt::entry!(main);

/// A page.
const PAGE: usize = 4096;

/// The status every start ends with: `EX_SOFTWARE`, a driver's own failure.
const DIED: i32 = 70;

/// What every line starts with.
const SAID: &str = "crash-at-start:";

fn main(bootstrap: Bootstrap) -> i32 {
    let Some(boot) = bootstrap else {
        xaqu_rt::err!("{SAID} no bootstrap channel");
        return DIED;
    };
    let _ = boot.wait_one(Signals::READABLE, Deadline::Never);
    let mut bytes = [0_u8; START_BYTES];
    let mut handles = [Handle::INVALID; 1];
    let Ok(received) = boot.read(&mut bytes, &mut handles) else {
        xaqu_rt::err!("{SAID} no START");
        return DIED;
    };
    let Ok(StartMessage::Start(start)) =
        StartMessage::decode(bytes.get(..received.bytes).unwrap_or_default())
    else {
        xaqu_rt::err!("{SAID} not a START");
        return DIED;
    };
    if received.handles != 1 {
        xaqu_rt::err!("{SAID} START carried {} handles, not 1", received.handles);
        return DIED;
    }
    let device = Device::from_owned(OwnedHandle::from_raw(Kernel, handles[0]));
    let location = start.location;
    let pinned = vmo::create(Kernel, PAGE).ok().and_then(|memory| {
        device
            .pin(&memory, 0, PAGE, PinAccess::ReadWrite)
            .ok()
            .map(|pin| (memory, pin))
    });
    let millis = xaqu_rt::linux::monotonic_nanos().unwrap_or(0) / 1_000_000;
    let pin = if pinned.is_some() {
        "pinned 1 page"
    } else {
        "a pin of 1 page was refused"
    };
    xaqu_rt::err!("{SAID} {location:#010x} at {millis} ms: {pin}; dying at my start");
    // Held to the end: the death, not this program, gives the pin up, so the
    // kernel quarantines it as a dead driver's.
    let _held = core::mem::ManuallyDrop::new(pinned);
    DIED
}
