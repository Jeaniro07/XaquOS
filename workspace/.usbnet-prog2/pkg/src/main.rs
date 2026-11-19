//! `usbnet`: the class driver of one USB Ethernet interface, in a process
//! of its own.
//!
//! devmgr starts it for an interface `usbhid` exported whose identity its
//! table names -- a CDC-ECM or a CDC-NCM dongle -- with BIND on its
//! bootstrap channel: the device's number and the interface channel. The
//! host's HELLO waits in that channel with the shared VMO. Then:
//!
//! 1. both bulk endpoints opened as streams;
//! 2. the device brought up, by subclass: ECM's three class requests and
//!    the station's address out of the configuration descriptor, or NCM's
//!    block setup;
//! 3. one wait, on the port the interface channel is watched on, and on
//!    each wake the frames the device sent, and the frames the network side
//!    wants sent.
//!
//! The network side is a stand-in for now: every frame is echoed straight
//! back. See [`net`] for where the real transport goes.
//!
//! What it does is said on standard error, which is the console. The exit
//! status names the step that failed ([`Step`]).

#![no_std]
#![no_main]

mod link;
mod mode;
mod net;

use core::fmt::Write as _;

use xaqu_native_abi::handle::Handle;
use xaqu_native_abi::signals::Signals;
use xaqu_rt::Kernel;
use xaqu_rt::native::channel::Channel;
use xaqu_rt::native::handle::{Deadline, Object, OwnedHandle};
use xaqu_rt::native::port::{self, Port};
use xaqu_rt::native::vmo::Vmo;
use xaqu_rt::{Bootstrap, linux};
use xaqu_usb_serial::CHUNK_BYTES;
use xaqu_usbif::{IN_SLOT_BYTES, MAX_BYTES, Message};

use crate::link::Link;
use crate::mode::Protocol;
use crate::net::{Loopback, Network};

xaqu_rt::entry!(main);

/// The interface channel's port key.
const KEY_LINK: u64 = 1;
/// How long one wait lasts, in nanoseconds.
const WAIT_NANOS: u64 = 100_000_000;
/// The RNDIS subclass: the protocol this driver does not carry yet.
const RNDIS_SUBCLASS: u8 = 0x02;
/// The most text one line keeps.
const TEXT_BYTES: usize = 96;

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
    /// The device would not come up.
    Mode = 4,
    /// The interface's subclass is not one this driver speaks.
    Unsupported = 5,
    /// The interface is RNDIS, which this driver does not carry yet.
    Rndis = 6,
    /// The port or the wait could not be arranged.
    Events = 7,
}

fn main(bootstrap: Bootstrap) -> i32 {
    let Some(boot) = bootstrap else {
        return Step::Start as i32;
    };
    match run(&boot) {
        Ok(()) => 0,
        Err(step) => {
            xaqu_rt::err!("usbnet: stopped at {step:?}");
            step as i32
        }
    }
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

/// The driver of one interface: the link to the host, the protocol, the
/// network side, and the port the interface channel is watched on.
struct Driver {
    number: u8,
    link: Link,
    protocol: Protocol,
    net: Loopback,
    port: Port<Kernel>,
    transfer: Transfer,
    sequence: u16,
}

fn run(boot: &Channel<Kernel>) -> Result<(), Step> {
    let Some((Message::Bind { number }, interface)) = first(boot) else {
        return Err(Step::Start);
    };
    let channel = Channel::from_owned(interface);
    let Some((Message::Hello {
        identity,
        count,
        endpoints,
        ..
    }, vmo)) = first(&channel)
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
    let protocol = match Protocol::of(identity.subclass) {
        Some(protocol) => protocol,
        None if identity.subclass == RNDIS_SUBCLASS => {
            xaqu_rt::err!(
                "usbnet: serial {number}: RNDIS is a later step: the crate has the protocol, but this driver does not carry it yet"
            );
            return Err(Step::Rndis);
        }
        None => {
            xaqu_rt::err!(
                "usbnet: serial {number}: the subclass {:#04x} is not one this driver speaks",
                identity.subclass
            );
            return Err(Step::Unsupported);
        }
    };
    let mut link = Link::new(
        channel,
        Vmo::from_owned(vmo),
        inward.address,
        outward.address,
    );
    link.open_streams().map_err(|error| {
        xaqu_rt::err!("usbnet: {number}: the streams would not open: {error:?}");
        Step::Streams
    })?;
    let mac = mode::bring_up(&mut link, u16::from(identity.interface), protocol).map_err(|_| {
        xaqu_rt::err!("usbnet: {number}: the device would not come up");
        Step::Mode
    })?;
    xaqu_rt::info!(
        "usbnet: serial {number} {:04x}:{:04x} is up ({}), station {}",
        identity.vendor,
        identity.product,
        protocol.name(),
        mac_text(mac)
    );
    xaqu_rt::info!(
        "usbnet: serial {number}: the network side is a stand-in: every frame is echoed straight back"
    );
    let port = port::create(Kernel).map_err(|_| Step::Events)?;
    let driver = Driver {
        number,
        link,
        protocol,
        net: Loopback::new(),
        port,
        transfer: Transfer::new(),
        sequence: 0,
    };
    driver.serve()
}

impl Driver {
    /// Wait, move, and again, until the interface goes.
    fn serve(&mut self) -> Result<(), Step> {
        loop {
            self.link
                .channel
                .wait_async(
                    &self.port,
                    Signals::READABLE | Signals::PEER_CLOSED,
                    KEY_LINK,
                )
                .map_err(|_| Step::Events)?;
            let until = linux::monotonic_nanos()
                .unwrap_or(0)
                .saturating_add(WAIT_NANOS);
            let _ = self.port.wait(Deadline::At(until));
            while let Ok(packet) = self.port.wait(Deadline::At(0)) {
                let _ = packet.key;
            }
            self.link.take_messages();
            if self.link.is_gone() {
                xaqu_rt::info!("usbnet: serial {} unplugged", self.number);
                return Ok(());
            }
            let lost = self.link.take_lost();
            if lost > 0 {
                xaqu_rt::warn!("usbnet: serial {} lost {lost} bytes to a stalled endpoint", self.number);
            }
            self.pump_in();
            self.pump_out();
        }
    }

    /// The frames the device sent, to the network side.
    fn pump_in(&mut self) {
        let Driver {
            link,
            protocol,
            net,
            transfer,
            ..
        } = self;
        loop {
            let mut piece = [0_u8; CHUNK_BYTES];
            let Some(len) = link.take_chunk(&mut piece) else {
                return;
            };
            if !transfer.push(piece.get(..len).unwrap_or_default()) {
                continue;
            }
            let bytes = transfer.bytes();
            match protocol {
                Protocol::Ecm => {
                    if let Some(frame) = mode::ecm_frame(bytes) {
                        net.deliver(frame);
                    }
                }
                Protocol::Ncm => mode::ncm_frames(bytes, |frame| net.deliver(frame)),
            }
            transfer.clear();
        }
    }

    /// The frames the network side wants sent, to the device.
    fn pump_out(&mut self) {
        let Driver {
            link,
            protocol,
            net,
            sequence,
            ..
        } = self;
        let mut frame = [0_u8; net::MAX_FRAME];
        let Some(len) = net.take_outgoing(&mut frame) else {
            return;
        };
        let frame = frame.get(..len).unwrap_or_default();
        match protocol {
            Protocol::Ecm => {
                if link.room() < frame.len() {
                    return;
                }
                if let Err(error) = link.send(frame) {
                    xaqu_rt::warn!("usbnet: a frame was not sent: {error:?}");
                }
            }
            Protocol::Ncm => {
                let mut block = [0_u8; mode::MAX_BLOCK];
                let Some(size) = mode::ncm_block(frame, *sequence, &mut block) else {
                    return;
                };
                *sequence = sequence.wrapping_add(1);
                if link.room() < size {
                    return;
                }
                if let Err(error) = link.send(block.get(..size).unwrap_or_default()) {
                    xaqu_rt::warn!("usbnet: a block was not sent: {error:?}");
                }
            }
        }
    }
}

/// The bytes of one transfer, as they come in.
///
/// The host reads an endpoint in pieces of [`IN_SLOT_BYTES`], and only a
/// piece shorter than that ends a transfer, because only a short packet
/// does: a piece of the full size may be any part of a longer transfer.
struct Transfer {
    bytes: [u8; mode::MAX_BLOCK],
    len: usize,
}

impl Transfer {
    /// A transfer with nothing in it yet.
    const fn new() -> Self {
        Transfer {
            bytes: [0; mode::MAX_BLOCK],
            len: 0,
        }
    }

    /// Take a piece in; whether the transfer is complete.
    fn push(&mut self, piece: &[u8]) -> bool {
        let room = mode::MAX_BLOCK.saturating_sub(self.len);
        let take = piece.len().min(room);
        if let Some(slot) = self.bytes.get_mut(self.len..self.len + take) {
            slot.copy_from_slice(piece.get(..take).unwrap_or_default());
        }
        self.len += take;
        piece.len() < IN_SLOT_BYTES
    }

    /// The transfer's bytes so far.
    fn bytes(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or_default()
    }

    /// Forget the transfer, for the next one.
    fn clear(&mut self) {
        self.len = 0;
    }
}

/// A line's text, built in place: a native program has no allocator.
struct Text {
    bytes: [u8; TEXT_BYTES],
    len: usize,
}

impl Text {
    /// An empty text.
    const fn new() -> Self {
        Text {
            bytes: [0; TEXT_BYTES],
            len: 0,
        }
    }

    /// The text, as far as it is whole.
    fn as_str(&self) -> &str {
        core::str::from_utf8(self.bytes.get(..self.len).unwrap_or_default()).unwrap_or("")
    }
}

impl core::fmt::Write for Text {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let end = self.len + text.len();
        let slot = self.bytes.get_mut(self.len..end).ok_or(core::fmt::Error)?;
        slot.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

impl core::fmt::Display for Text {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The station's address, as the console shows it.
fn mac_text(mac: Option<[u8; 6]>) -> Text {
    let mut text = Text::new();
    let Some(mac) = mac else {
        return text;
    };
    for (index, byte) in mac.iter().enumerate() {
        if index > 0 {
            let _ = write!(text, ":");
        }
        let _ = write!(text, "{byte:02x}");
    }
    text
}
