//! The Intel HD Audio driver process: a ring-3 program that serves one
//! sound card to the kernel's audio core.
//!
//! An HD Audio controller is a host controller with a codec behind it. The
//! host's register block holds the CORB the verbs go out on, the RIRB the
//! codec's answers come back on, and one block of registers per stream; the
//! codec is a separate device, reachable only through those verbs.
//!
//! `xaqu-hda` knows the controller and the codec and is tested on the host:
//! the registers, the verbs, the codec's parameters, the path a pin's sound
//! takes, and the buffer descriptor list. This program is the rest:
//!
//! 1. The bootstrap channel carries START: the device, the driver's end of
//!    the sound control channel, and which of the device's base address
//!    registers holds the controller's register block.
//! 2. That register block is mapped, the controller is reset, and the CORB
//!    and the RIRB are given memory the controller can reach.
//! 3. The codec is asked about its nodes: which are pins, what device each
//!    pin belongs to, and how they are wired. The path from the converter
//!    to the pin is found and programmed: power, gain, the pin's control,
//!    and the amplifier.
//! 4. The card is published to the core with HELLO, and the core's buffer
//!    is pinned for the device. The stream is programmed with a buffer
//!    descriptor list and started.
//! 5. Each submission the core makes is waited for on the controller's
//!    interrupt and answered with ELAPSED; a halt is answered with HALTED,
//!    and a stop with STOPPED.
//!
//! The exit status names the step that failed ([`Step`]), 0 a clean STOP.

#![no_std]
#![no_main]

use core::sync::atomic::{Ordering, fence};

use xaqu_blkring::control::Block as StartBlock;
use xaqu_driver::Step;
use xaqu_driver::dma::Dma;
use xaqu_driver::mmio::Block as MmioBlock;
use xaqu_hda::bdl;
use xaqu_hda::codec;
use xaqu_hda::control;
use xaqu_hda::controller::{self, Controller, Mmio, register, stream};
use xaqu_hda::corb::{Corb, Memory, Rirb};
use xaqu_hda::path;
use xaqu_native_abi::handle::Handle;
use xaqu_native_abi::rights::Requested;
use xaqu_native_abi::signals::Signals;
use xaqu_rt::native::channel::Channel;
use xaqu_rt::native::device::{Device, Interrupt};
use xaqu_rt::native::handle::{Deadline, Object, OwnedHandle};
use xaqu_rt::native::pin::{PinAccess, device_address};
use xaqu_rt::native::port::{self, Port};
use xaqu_rt::native::vmo::Vmo;
use xaqu_rt::{Bootstrap, Kernel};
use xaqu_sndctl::message::{self as snd, MAX_PUBLISHED, MAX_STREAMS, Offer};

xaqu_rt::entry!(main);

/// The base address register the controller's registers are in.
const BAR: u8 = 0;

/// The most bytes of the controller's register block that are mapped.
const REGISTER_BYTES: u32 = 0x2000;

/// The CORB's entries: the specification's full ring.
const CORB_ENTRIES: u16 = 256;

/// The RIRB's entries: the specification's full ring.
const RIRB_ENTRIES: u16 = 256;

/// How long a wait for a codec's answer looks before it gives up.
const ANSWER_SPINS: u32 = 100_000;

/// How long a nap between the looks lasts, in nanoseconds.
const NAP_NANOS: u64 = 10_000;

/// The most nodes of a codec this driver reads.
const MAX_NODES: usize = 32;

/// The most connections of a node this driver reads.
const MAX_CONNECTIONS: usize = 32;

/// Port keys.
const KEY_INTERRUPT: u64 = 1;

/// The stream the card plays through: the first output stream.
const PLAYBACK_STREAM: u8 = 4;

/// The codec's address: the first one.
const CODEC: u8 = 0;

/// The audio format the card offers: sixteen bits, two channels.
const FORMAT_S16_LE: u64 = 1 << 2;

/// The power state a widget is woken to.
const POWER_D0: u8 = 0;

/// The amplifier's gain, in the codec's own steps.
const GAIN: u8 = 0x20;

fn main(bootstrap: Bootstrap) -> i32 {
    let Some(boot) = bootstrap else {
        return Step::Start.status();
    };
    match run(&boot) {
        Ok(()) => 0,
        Err(step) => step.status(),
    }
}

// ---------------------------------------------------------------------------
// The registers and the memory
// ---------------------------------------------------------------------------

/// The controller's register block, mapped.
struct Regs(MmioBlock);

impl Mmio for Regs {
    fn read8(&self, offset: u32) -> u8 {
        self.0.read::<u8>(offset)
    }

    fn read16(&self, offset: u32) -> u16 {
        self.0.read::<u16>(offset)
    }

    fn read32(&self, offset: u32) -> u32 {
        self.0.read::<u32>(offset)
    }

    fn write8(&mut self, offset: u32, value: u8) {
        self.0.write::<u8>(offset, value);
    }

    fn write16(&mut self, offset: u32, value: u16) {
        self.0.write::<u16>(offset, value);
    }

    fn write32(&mut self, offset: u32, value: u32) {
        self.0.write::<u32>(offset, value);
    }
}

/// A memory the controller can reach, read and written by offset.
struct Ring(Dma);

impl Memory for Ring {
    fn read32(&self, offset: u32) -> u32 {
        let low = u32::from(self.0.read_u16(offset as usize));
        let high = u32::from(self.0.read_u16(offset as usize + 2));
        low | (high << 16)
    }

    fn write32(&mut self, offset: u32, value: u32) {
        self.0.write_u16(offset as usize, (value & 0xFFFF) as u16);
        self.0.write_u16(offset as usize + 2, (value >> 16) as u16);
        fence(Ordering::SeqCst);
    }
}

/// The codec's nodes, as the driver read them.
struct Nodes {
    count: u8,
    caps: [u32; MAX_NODES],
    pins: [u32; MAX_NODES],
    defaults: [u32; MAX_NODES],
    conns: [codec::Connections; MAX_NODES],
}

impl Nodes {
    fn new() -> Self {
        Self {
            count: 0,
            caps: [0; MAX_NODES],
            pins: [0; MAX_NODES],
            defaults: [0; MAX_NODES],
            conns: [codec::Connections::default(); MAX_NODES],
        }
    }
}

impl path::Nodes for Nodes {
    fn count(&self) -> u8 {
        self.count
    }

    fn kind(&self, id: u8) -> codec::NodeType {
        self.caps
            .get(id as usize)
            .map(|caps| codec::NodeType::of(*caps))
            .unwrap_or(codec::NodeType::Other)
    }

    fn connections(&self, id: u8) -> codec::Connections {
        self.conns.get(id as usize).copied().unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// The run
// ---------------------------------------------------------------------------

/// What START gave.
struct Started {
    start: control::Start,
    device: Device<Kernel>,
    control: Channel<Kernel>,
}

fn started(boot: &Channel<Kernel>) -> Result<Started, Step> {
    let _ = boot
        .wait_one(Signals::READABLE, Deadline::Never)
        .map_err(|_| Step::Start)?;
    let mut bytes = [0_u8; control::START_BYTES];
    let mut handles = [Handle::INVALID; 2];
    let received = boot
        .read(&mut bytes, &mut handles)
        .map_err(|_| Step::Start)?;
    if received.handles != 2 {
        return Err(Step::Start);
    }
    let start = control::Start::decode(bytes.get(..received.bytes).unwrap_or_default())
        .ok_or(Step::Start)?;
    let device = Device::from_owned(OwnedHandle::from_raw(Kernel, handles[0]));
    let control = Channel::from_owned(OwnedHandle::from_raw(Kernel, handles[1]));
    Ok(Started {
        start,
        device,
        control,
    })
}

/// The controller's register block, mapped from its base address register.
fn registers(device: &Device<Kernel>) -> Result<MmioBlock, Step> {
    let info = device.info().map_err(|_| Step::Registers)?;
    for index in 0..info.apertures {
        let Ok(index) = usize::try_from(index) else {
            break;
        };
        let Ok(aperture) = device.aperture(index) else {
            continue;
        };
        if aperture.bar != BAR {
            continue;
        }
        let length = aperture.len.min(u64::from(REGISTER_BYTES)) as u32;
        let block = StartBlock {
            phys: aperture.phys,
            offset: aperture.offset as u32,
            length,
        };
        return MmioBlock::map(device, &block);
    }
    Err(Step::Registers)
}

/// The driver, with everything it holds.
struct Hda {
    controller: Controller<Regs>,
    corb: Ring,
    rirb: Ring,
    /// The RIRB entry the last answer went into.
    last_answer: u8,
    device: Device<Kernel>,
    interrupt: Interrupt<Kernel>,
    port: Port<Kernel>,
    control: Channel<Kernel>,
}

impl Hda {
    /// Send a verb and wait for the codec's answer.
    fn verb(&mut self, verb: u32) -> Result<u32, Step> {
        {
            let read = self.controller.mmio().read8(register::CORBRP);
            let mut corb = Corb::new(&mut self.corb, CORB_ENTRIES);
            corb.set_read_pointer(u16::from(read));
            let at = corb.push(verb).map_err(|_| Step::Faulted)?;
            self.controller
                .mmio_mut()
                .write8(register::CORBWP, at as u8);
        }
        for _ in 0..ANSWER_SPINS {
            let written = self.controller.mmio().read8(register::RIRBWP);
            if written != self.last_answer {
                self.last_answer = written;
                let rirb = Rirb::new(&self.rirb, RIRB_ENTRIES);
                return rirb.answer(u16::from(written)).ok_or(Step::Faulted);
            }
            self.nap();
        }
        Err(Step::Faulted)
    }

    /// Read a parameter of a node.
    fn parameter(&mut self, nid: u8, parameter: u8) -> Result<u32, Step> {
        let verb = codec::get_parameter(CODEC, nid, parameter);
        self.verb(verb)
    }

    /// Sleep a little, so a codec that answers late is not missed.
    fn nap(&self) {
        if let Ok(now) = xaqu_rt::linux::monotonic_nanos() {
            let _ = self.port.wait(Deadline::At(now.saturating_add(NAP_NANOS)));
        } else {
            core::hint::spin_loop();
        }
    }

    /// The node's connection list.
    fn connections(&mut self, nid: u8) -> Result<codec::Connections, Step> {
        let length = self.parameter(nid, codec::parameter::CONNLIST_LEN)? & 0x7F;
        let mut bytes = [0_u8; MAX_CONNECTIONS];
        let mut at = 0_usize;
        while at < bytes.len() && (at as u32) < length {
            let word = self.parameter(nid, codec::parameter::CONNECTION_LIST)?;
            let chunk = word.to_le_bytes();
            if let Some(slot) = bytes.get_mut(at..at + chunk.len()) {
                slot.copy_from_slice(&chunk);
            }
            at += chunk.len();
        }
        Ok(codec::Connections::unpack(&bytes, length as usize))
    }
}

/// Read the codec's nodes: what they are, and how they are wired.
fn read_nodes(hda: &mut Hda) -> Result<Nodes, Step> {
    let mut nodes = Nodes::new();
    let count = hda.parameter(0, codec::parameter::NODE_COUNT)? & 0x7F;
    nodes.count = u8::try_from(count.min(MAX_NODES as u32)).unwrap_or(0);
    for id in 0..nodes.count {
        let nid = id;
        let caps = hda.parameter(nid, codec::parameter::WIDGET_CAP)?;
        if let Some(slot) = nodes.caps.get_mut(id as usize) {
            *slot = caps;
        }
        if codec::NodeType::of(caps) == codec::NodeType::Pin {
            let pins = hda.parameter(nid, codec::parameter::PIN_CAP)?;
            let defaults = hda.verb(codec::get_config_default(CODEC, nid));
            if let Some(slot) = nodes.pins.get_mut(id as usize) {
                *slot = pins;
            }
            if let Some(slot) = nodes.defaults.get_mut(id as usize) {
                *slot = defaults.unwrap_or_default();
            }
        }
        let conns = hda.connections(nid)?;
        if let Some(slot) = nodes.conns.get_mut(id as usize) {
            *slot = conns;
        }
    }
    Ok(nodes)
}

/// The pin that plays the card's sound out, if there is one.
///
/// The speakers come first, then the headphones, then a line-level socket:
/// the order a laptop's sound is wanted in.
fn output_pin(nodes: &Nodes) -> Option<u8> {
    let wanted = [
        codec::Jack::Speaker,
        codec::Jack::Headphone,
        codec::Jack::LineOut,
    ];
    for jack in wanted {
        for id in 0..nodes.count {
            let Some(default) = nodes.defaults.get(id as usize) else {
                continue;
            };
            let Some(caps) = nodes.pins.get(id as usize) else {
                continue;
            };
            let default = codec::ConfigDefault::of(*default);
            let caps = codec::PinCapabilities::of(*caps);
            if path::serves(&caps, &default, jack) {
                return Some(id);
            }
        }
    }
    None
}

/// Wake, connect and gain the widgets on the path, and open the pin.
fn program_path(hda: &mut Hda, path: &path::Path) -> Result<(), Step> {
    let converter = path.converter;
    let amplifier = path.amplifier;
    let mixer = path.mixer;
    let pin = path.pin;
    let _ = hda.verb(codec::set_power(CODEC, converter, POWER_D0));
    if let Some(id) = amplifier {
        let _ = hda.verb(codec::set_power(CODEC, id, POWER_D0));
        let _ = hda.verb(codec::set_amp_gain(CODEC, id, GAIN, false));
    }
    if let Some(id) = mixer {
        let _ = hda.verb(codec::set_power(CODEC, id, POWER_D0));
        let _ = hda.verb(codec::set_connection_select(CODEC, id, 0));
    }
    let _ = hda.verb(codec::set_power(CODEC, pin, POWER_D0));
    let control = codec::PINCTL_OUT_EN | codec::PINCTL_HP_EN;
    let _ = hda.verb(codec::set_pin_control(CODEC, pin, control));
    let _ = hda.verb(codec::set_eapd(CODEC, pin, true));
    Ok(())
}

fn run(boot: &Channel<Kernel>) -> Result<(), Step> {
    let Started {
        start,
        device,
        control,
    } = started(boot)?;

    let regs = registers(&device)?;
    let corb = Dma::new(&device, 1)?;
    let rirb = Dma::new(&device, 1)?;
    let corb_address = *corb.device_pages().first().ok_or(Step::Memory)?;
    let rirb_address = *rirb.device_pages().first().ok_or(Step::Memory)?;

    let mut controller = Controller::new(Regs(regs));
    controller.reset(ANSWER_SPINS).map_err(|_| Step::Device)?;
    controller
        .mmio_mut()
        .write32(register::CORBLBASE, corb_address as u32);
    controller
        .mmio_mut()
        .write32(register::CORBUBASE, (corb_address >> 32) as u32);
    controller
        .mmio_mut()
        .write32(register::RIRBLBASE, rirb_address as u32);
    controller
        .mmio_mut()
        .write32(register::RIRBUBASE, (rirb_address >> 32) as u32);
    controller.open_corb();
    controller.open_rirb();
    controller.mmio_mut().write16(register::RINTCNT, 1);

    let interrupt = device.interrupt(0).map_err(|_| Step::Events)?;
    let port = port::create(Kernel).map_err(|_| Step::Events)?;
    interrupt
        .bind(&port, KEY_INTERRUPT)
        .map_err(|_| Step::Events)?;

    let mut hda = Hda {
        controller,
        corb: Ring(corb),
        rirb: Ring(rirb),
        last_answer: 0,
        device,
        interrupt,
        port,
        control,
    };

    let nodes = read_nodes(&mut hda)?;
    let Some(pin) = output_pin(&nodes) else {
        return Ok(());
    };
    let Some(found) = path::path(&nodes, pin) else {
        return Err(Step::Faulted);
    };
    program_path(&mut hda, &found)?;

    let card = publish(&mut hda, &start)?;
    let _ = card;
    serve(&mut hda)
}

/// Tell the core what the card offers, and take what it publishes.
fn publish(hda: &mut Hda, start: &control::Start) -> Result<(), Step> {
    let rates =
        snd::rate_bit(48_000).unwrap_or_default() | snd::rate_bit(44_100).unwrap_or_default();
    let offer = Offer {
        direction: snd::DIRECTION_PLAYBACK,
        channels_min: 2,
        channels_max: 2,
        rates,
        formats: FORMAT_S16_LE,
    };
    let mut offers = [Offer::default(); MAX_STREAMS];
    if let Some(slot) = offers.get_mut(0) {
        *slot = offer;
    }
    let hello = snd::Hello {
        version: snd::VERSION,
        location: start.location,
        streams: 1,
        offers,
    };
    let port_share = hda
        .port
        .as_owned()
        .duplicate(Requested::Exactly(snd::PORT_RIGHTS))
        .map_err(|_| Step::Hello)?;
    hda.control
        .write_with(snd::Message::Hello(hello).encode().as_bytes(), [port_share])
        .map_err(|_| Step::Hello)?;
    let _ = hda
        .control
        .wait_one(Signals::READABLE, Deadline::Never)
        .map_err(|_| Step::Hello)?;
    let mut bytes = [0_u8; snd::MAX_BYTES];
    let mut handles = [Handle::INVALID; 1 + MAX_PUBLISHED];
    let received = hda
        .control
        .read(&mut bytes, &mut handles)
        .map_err(|_| Step::Hello)?;
    let decoded = snd::Message::decode(bytes.get(..received.bytes).unwrap_or_default())
        .map_err(|_| Step::Control)?;
    let ready = match decoded {
        snd::Message::Ready(ready) => ready,
        _ => return Err(Step::Control),
    };
    if ready.published == 0 {
        return Err(Step::Control);
    }
    let Some(stream) = ready.streams.first().copied() else {
        return Err(Step::Control);
    };
    let Some(handle) = handles.get(1).copied() else {
        return Err(Step::Control);
    };
    let vmo = Vmo::from_owned(OwnedHandle::from_raw(Kernel, handle));
    start_stream(hda, &vmo, &stream)
}

/// Program the stream: the format, the buffer and the list that plays it.
fn start_stream(
    hda: &mut Hda,
    vmo: &Vmo<Kernel>,
    stream_config: &snd::Published,
) -> Result<(), Step> {
    let bytes = stream_config.buffer_bytes as usize;
    let pages = bytes.div_ceil(4096).max(1);
    let pin = hda
        .device
        .pin(vmo, 0, bytes, PinAccess::ReadOnly)
        .map_err(|_| Step::Memory)?;
    let mut raw = [[0_u8; 8]; 128];
    let asked = raw.get_mut(..pages).ok_or(Step::Memory)?;
    let got = pin.addresses(asked).map_err(|_| Step::Memory)?;
    if !got.is_complete() {
        return Err(Step::Memory);
    }
    let Some(first) = raw.first().copied() else {
        return Err(Step::Memory);
    };
    let address = device_address(first);

    let mut list = Dma::new(&hda.device, 1)?;
    let list_address = *list.device_pages().first().ok_or(Step::Memory)?;
    let entry = bdl::Entry {
        address,
        bytes: stream_config.period_bytes,
        interrupt: true,
    };
    for (index, byte) in entry.encode().iter().enumerate() {
        list.write_u8(index, *byte);
    }
    fence(Ordering::SeqCst);

    let base = controller::stream_base(PLAYBACK_STREAM);
    hda.controller.mmio_mut().write16(base + stream::LVI, 0);
    hda.controller
        .mmio_mut()
        .write32(base + stream::CBL, stream_config.buffer_bytes);
    hda.controller
        .mmio_mut()
        .write16(base + stream::FORMAT, controller::FORMAT_48K_S16_2CH);
    hda.controller
        .set_stream_bdl_base(PLAYBACK_STREAM, list_address);
    let tag = stream_config.stream << controller::ST_CTL_TAG_SHIFT;
    let ctl = controller::ST_CTL_RUN | tag;
    hda.controller.mmio_mut().write32(base + stream::CTL, ctl);
    Ok(())
}

/// Answer the core: each submission when the controller says it is played.
fn serve(hda: &mut Hda) -> Result<(), Step> {
    loop {
        let _ = hda
            .control
            .wait_one(Signals::READABLE, Deadline::Never)
            .map_err(|_| Step::Control)?;
        let mut bytes = [0_u8; snd::MAX_BYTES];
        let mut handles = [Handle::INVALID; 1 + MAX_PUBLISHED];
        let received = hda
            .control
            .read(&mut bytes, &mut handles)
            .map_err(|_| Step::Control)?;
        let decoded = snd::Message::decode(bytes.get(..received.bytes).unwrap_or_default())
            .map_err(|_| Step::Control)?;
        match decoded {
            snd::Message::Submit(submit) => {
                let _ = hda.port.wait(Deadline::Never);
                let _ = hda.interrupt.ack();
                let elapsed = snd::Elapsed {
                    stream: submit.stream,
                    sequence: submit.sequence,
                    played: true,
                    latency_bytes: 0,
                };
                hda.control
                    .write(snd::Message::Elapsed(elapsed).encode().as_bytes())
                    .map_err(|_| Step::Control)?;
            }
            snd::Message::Halt { stream } => {
                let halted = snd::Message::Halted {
                    stream,
                    unplayed: 0,
                };
                hda.control
                    .write(halted.encode().as_bytes())
                    .map_err(|_| Step::Control)?;
            }
            snd::Message::Stop => {
                let base = controller::stream_base(PLAYBACK_STREAM);
                hda.controller
                    .mmio_mut()
                    .write32(base + stream::CTL, controller::ST_CTL_RESET);
                hda.control
                    .write(snd::Message::Stopped.encode().as_bytes())
                    .map_err(|_| Step::Control)?;
                return Ok(());
            }
            _ => {}
        }
    }
}
