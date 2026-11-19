//! `usbserial`: the class driver of one USB serial interface, in a process
//! of its own (`docs/USB-SERIAL.md` §6.1).
//!
//! devmgr starts it for an interface `usbhid` exported whose identity its
//! table names -- an FTDI UART or a USB-Blaster -- with BIND on its bootstrap
//! channel: the device's number and the interface channel. The host's
//! HELLO waits in that channel with the shared VMO. Then:
//!
//! 1. both bulk endpoints opened as streams, and the chip set up as Linux's
//!    `ftdi_sio` sets it up at open (`xaqu_usb_serial::port`);
//! 2. a pseudoterminal ([`tty`]), its slave linked as `/tmp/ttyUSB<N>`, or
//!    for a Blaster `/tmp/jtag<N>`, which takes JTAG command lines;
//! 3. one wait, a `ppoll` on the port the interface channel is watched on
//!    and on the terminal's master, and on each wake the settings, the
//!    program's bytes out and the chip's bytes in.
//!
//! The interface channel closing is the device going, or `usbhid`: the
//! terminal hangs up and the driver ends with 0. Its own death is heard by
//! `usbhid`, which attaches the interface again; devmgr starts a new driver
//! under the same number, whose budget its restart policy keeps.
//!
//! What it does is said on standard error, which is the console. The exit
//! status names the step that failed ([`Step`]).

#![no_std]
#![no_main]

mod link;
mod tty;

use core::fmt::Write as _;

use xaqu_native_abi::handle::Handle;
use xaqu_native_abi::signals::Signals;
use xaqu_rt::native::channel::Channel;
use xaqu_rt::native::handle::{Deadline, Object, OwnedHandle};
use xaqu_rt::native::port::{self, Port};
use xaqu_rt::native::vmo::Vmo;
use xaqu_rt::{Bootstrap, Kernel, linux};
use xaqu_usb_serial::blaster::{self, Command, Jtag, MAX_TAPS, SCAN_BYTES, Tap};
use xaqu_usb_serial::ftdi::Role;
use xaqu_usb_serial::mpsse::{self, Outcome};
use xaqu_usb_serial::port::{Cable, Identity, Serial};
use xaqu_usbif::{MAX_BYTES, Message};

use crate::link::Link;
use crate::tty::{Kind, Tty};

xaqu_rt::entry!(main);

/// The interface channel's port key.
const KEY_LINK: u64 = 1;
/// What one look moves each way at most.
const PUMP_BYTES: usize = 4096;
/// The longest command line kept.
const LINE_BYTES: usize = 256;
/// The longest answer line.
const ANSWER_BYTES: usize = 192;
/// `POLLIN`.
const POLLIN: i16 = 0x001;

/// Where a run stopped, as the exit status.
#[derive(Clone, Copy, Debug)]
#[repr(i32)]
enum Step {
    /// No bootstrap channel, or its first message was not BIND.
    Start = 1,
    /// The interface channel's first message was not HELLO, or HELLO named
    /// no bulk endpoint each way.
    Hello = 2,
    /// The streams would not open.
    Streams = 3,
    /// The chip would not be set up.
    Chip = 4,
    /// No terminal could be made.
    Terminal = 5,
    /// The port or the wait could not be arranged.
    Events = 6,
}

fn main(bootstrap: Bootstrap) -> i32 {
    let Some(boot) = bootstrap else {
        return Step::Start as i32;
    };
    match run(&boot) {
        Ok(()) => 0,
        Err(step) => {
            xaqu_rt::err!("usbserial: stopped at {step:?}");
            step as i32
        }
    }
}

/// Whether `path`, NUL-terminated, names a file root owns, not following a
/// last symbolic link: `statx`'s `stx_uid`, the same place on every
/// architecture.
fn owned_by_root(path: &[u8]) -> bool {
    const AT_FDCWD: isize = -100;
    const AT_SYMLINK_NOFOLLOW: usize = 0x100;
    const STATX_UID: usize = 0x8;
    let mut statx = [0_u8; 256];
    // SAFETY: `path` is NUL-terminated and `statx` is 256 bytes, what the
    // call writes, both alive for the call.
    let found = unsafe {
        linux::call(
            linux::numbers::STATX,
            [
                AT_FDCWD as usize,
                path.as_ptr().addr(),
                AT_SYMLINK_NOFOLLOW,
                STATX_UID,
                statx.as_mut_ptr().addr(),
                0,
            ],
        )
    };
    found.is_ok() && statx.get(20..24) == Some(&[0, 0, 0, 0])
}

/// The first message on `channel` and its one handle.
fn first(channel: &Channel<Kernel>) -> Option<(Message, OwnedHandle<Kernel>)> {
    let _ = channel.wait_one(Signals::READABLE, Deadline::Never).ok()?;
    let mut bytes = [0_u8; MAX_BYTES];
    let mut handles = [Handle::INVALID; 1];
    let received = channel.read(&mut bytes, &mut handles).ok()?;
    let handle = OwnedHandle::from_raw(Kernel, *handles.first()?);
    if received.handles != 1 {
        return None;
    }
    let message = Message::decode(bytes.get(..received.bytes).unwrap_or_default()).ok()?;
    Some((message, handle))
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

/// An answer line, built in place.
struct Answer {
    bytes: [u8; ANSWER_BYTES],
    len: usize,
}

impl core::fmt::Write for Answer {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let end = self.len + text.len();
        let slot = self.bytes.get_mut(self.len..end).ok_or(core::fmt::Error)?;
        slot.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

struct Driver {
    number: u8,
    link: Link,
    serial: Serial,
    tty: Tty,
    port: Port<Kernel>,
    port_fd: i32,
    armed: bool,
    /// The modem lines last said on the console.
    modem_said: Option<u8>,
}

fn run(boot: &Channel<Kernel>) -> Result<(), Step> {
    let Some((Message::Bind { number }, interface)) = first(boot) else {
        return Err(Step::Start);
    };
    let channel = Channel::from_owned(interface);
    let Some((
        Message::Hello {
            identity,
            count,
            endpoints,
            ..
        },
        vmo,
    )) = first(&channel)
    else {
        return Err(Step::Hello);
    };
    let endpoints = endpoints.get(..count).unwrap_or_default();
    let bulk = |inward: bool| {
        endpoints
            .iter()
            .find(|endpoint| endpoint.is_bulk() && endpoint.is_in() == inward)
            .copied()
    };
    let (Some(inward), Some(outward)) = (bulk(true), bulk(false)) else {
        return Err(Step::Hello);
    };
    let mut link = Link::new(
        channel,
        Vmo::from_owned(vmo),
        inward.address,
        outward.address,
    );
    link.open_streams().map_err(|error| {
        xaqu_rt::err!("usbserial: {number}: the streams would not open: {error:?}");
        Step::Streams
    })?;
    let chip = Identity {
        vendor: identity.vendor,
        product: identity.product,
        release: identity.release,
        interface: identity.interface,
        max_packet: inward.max_packet,
    };
    let serial = Serial::open(&mut link, &chip).map_err(|error| {
        xaqu_rt::err!(
            "usbserial: {number}: {:04x}:{:04x} would not be set up: {error:?}",
            identity.vendor,
            identity.product
        );
        Step::Chip
