//! The I2C-HID touchpad driver process: a touchpad on an Intel Serial I/O
//! (LPSS) I2C controller, served to the kernel's input core.
//!
//! A touchpad on an I2C bus is a HID device that keeps its registers
//! elsewhere: it keeps a *HID descriptor* at register 0, and that
//! descriptor says where its report descriptor is, where an input report is
//! read from, and how long each may be. The driver reads that descriptor
//! first, then the report descriptor, and learns from it what the device
//! sends: which axes, which buttons.
//!
//! The bus itself is the Intel Serial I/O (LPSS) I2C controller, a PCI
//! function whose register block this process maps and drives itself
//! (`xaqu-lpss-i2c`), because the kernel has no I2C subsystem of its own
//! and must not be touched for one.
//!
//! # Why the driver polls
//!
//! The touchpad's interrupt is a GPIO whose number is only known once the
//! platform has filled the GPIO group bases, so this driver cannot ask for
//! it. The driver therefore polls the touchpad every eight milliseconds and
//! sends the core an event batch only when the report has changed. When the
//! platform's GPIO groups are known, the interrupt path replaces the poll:
//! the same read of the input register, done when the platform says the
//! touchpad moved, with the poll as the fallback.
//!
//! The exit status is the [`Step`] that failed, 0 a clean stop.

#![no_std]
#![no_main]

use xaqu_blkring::control::{Block as StartBlock, Start};
use xaqu_driver::mmio::Block as MmioBlock;
use xaqu_driver::start::{self, Started};
use xaqu_driver::Step;
use xaqu_i2c_hid::{DESCRIPTOR_BYTES, Descriptor};
use xaqu_inputctl::message::{self, DeviceId, Events, Hello, Message};
use xaqu_linux_abi::input::BUS_I2C;
use xaqu_lpss_i2c::controller::{Clock, Controller, Error, Memory};
use xaqu_native_abi::handle::Handle;
use xaqu_native_abi::rights::Requested;
use xaqu_native_abi::signals::Signals;
use xaqu_rt::native::channel::{Channel, ReadError};
use xaqu_rt::native::device::Device;
use xaqu_rt::native::error::Error as RtError;
use xaqu_rt::native::handle::{Deadline, Object, OwnedHandle};
use xaqu_rt::native::port::{self, Port};
use xaqu_rt::{Bootstrap, Kernel};
use xaqu_usb_host::hid::{self, EventBuf, Interpreter};
use xaqu_usb_host::report;

xaqu_rt::entry!(main);

/// The base address register the controller's registers are in.
const BAR: u8 = 0;

/// The most bytes of the controller's register block that are mapped.
const REGISTER_BYTES: u32 = 0x1000;

/// The touchpad's address on the bus: what the laptop's own ACPI table
/// gives for `\_SB.PC00.I2C0.TPD0`.
const TOUCHPAD: u8 = 0x5D;

/// The register every HID device keeps its HID descriptor at.
const HID_DESCRIPTOR_REGISTER: u16 = 0x0000;

/// How long a transfer on the bus may take, in nanoseconds.
const TRANSFER_NANOS: u64 = 100_000_000;

/// How often the touchpad is asked for a report, in nanoseconds.
const POLL_NANOS: u64 = 8_000_000;

/// The most bytes of a report descriptor this driver keeps.
const MAX_REPORT_DESCRIPTOR: usize = 512;

/// The most bytes of an input report this driver keeps.
const MAX_INPUT_REPORT: usize = 64;

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
// The registers and the clock
// ---------------------------------------------------------------------------

/// The controller's register block, mapped.
struct Registers(MmioBlock);

impl Memory for Registers {
    fn read_u32(&self, offset: u32) -> u32 {
        self.0.read::<u32>(offset)
    }

    fn write_u32(&mut self, offset: u32, value: u32) {
        self.0.write::<u32>(offset, value);
    }
}

/// The platform's monotonic clock, in nanoseconds.
struct Monotonic;

impl Clock for Monotonic {
    fn now(&mut self) -> u64 {
        xaqu_rt::linux::monotonic_nanos().unwrap_or(0)
    }
}

/// The controller's register block, mapped from its base address register.
fn map_registers(device: &Device<Kernel>) -> Result<MmioBlock, Step> {
    let info = device.info().map_err(|_| Step::Registers)?;
    for index in 0..info.apertures {
        let Ok(index) = usize::try_from(index) else {
            break;
        };
        let Ok(aperture) = device.aperture(index) else {
            continue;
        };
        if aperture.bar != BAR || aperture.offset != 0 {
            continue;
        }
        let length =
            u32::try_from(aperture.len.min(u64::from(REGISTER_BYTES))).unwrap_or(u32::MAX);
        let block = StartBlock {
            phys: aperture.phys,
            offset: 0,
            length,
        };
        return MmioBlock::map(device, &block);
    }
    Err(Step::Registers)
}

// ---------------------------------------------------------------------------
// The touchpad
// ---------------------------------------------------------------------------

/// The touchpad, as the driver reads it.
struct Touchpad {
    /// Its HID descriptor: where its registers are.
    descriptor: Descriptor,
    /// The reader of its reports.
    interpreter: Interpreter,
}

/// Read the touchpad's HID descriptor and its report descriptor.
///
/// The HID descriptor is at register 0 and says where everything else is;
/// the report descriptor says what the reports hold.
fn describe<M: Memory, C: Clock>(
    controller: &mut Controller<M, C>,
) -> Result<Touchpad, Step> {
    let mut bytes = [0_u8; DESCRIPTOR_BYTES];
    read_register(controller, HID_DESCRIPTOR_REGISTER, &mut bytes).map_err(|_| Step::Device)?;
    let descriptor = Descriptor::parse(&bytes).map_err(|_| Step::Device)?;
    let length = usize::from(descriptor.report_desc_length).min(MAX_REPORT_DESCRIPTOR);
    let mut report_bytes = [0_u8; MAX_REPORT_DESCRIPTOR];
    let Some(slice) = report_bytes.get_mut(..length) else {
        return Err(Step::Device);
    };
    read_register(controller, descriptor.report_desc_register, slice).map_err(|_| Step::Device)?;
    let layout = report::Descriptor::parse(slice).map_err(|_| Step::Device)?;
    Ok(Touchpad {
        descriptor,
        interpreter: Interpreter::new(layout),
    })
}

/// Read `out`'s bytes from the touchpad's register `register`.
///
/// A register is read by writing its number and then reading the bytes with
/// no stop between: the device answers in the same transaction.
fn read_register<M: Memory, C: Clock>(
    controller: &mut Controller<M, C>,
    register: u16,
    out: &mut [u8],
) -> Result<(), Error> {
    let number = [
        u8::try_from(register & 0xFF).unwrap_or(0),
        u8::try_from(register >> 8).unwrap_or(0),
    ];
    controller.write_bytes(TOUCHPAD, &number, false)?;
    controller.read_bytes(TOUCHPAD, out, true)
}

/// Say what the touchpad said about itself.
fn say(touchpad: &Touchpad) {
    let descriptor = &touchpad.descriptor;
    xaqu_rt::notice!(
        "i2c-hid  the touchpad at 0x5D answers: a HID descriptor of {} bytes, a report \
         descriptor of {} bytes at register 0x{:04X}, an input report at register 0x{:04X} \
         of up to {} bytes",
        descriptor.desc_length,
        descriptor.report_desc_length,
        descriptor.report_desc_register,
        descriptor.input_register,
        descriptor.max_input_length
    );
    xaqu_rt::notice!(
        "i2c-hid  vendor 0x{:04X}, product 0x{:04X}, {} fields in its report descriptor, \
         a {}",
        descriptor.vendor_id,
        descriptor.product_id,
        touchpad.interpreter.descriptor().fields().len(),
        touchpad.interpreter.kind().word()
    );
}

// ---------------------------------------------------------------------------
// The run
// ---------------------------------------------------------------------------

fn run(boot: &Channel<Kernel>) -> Result<(), Step> {
    let Started {
        start,
        device,
        control,
    } = start::read(boot)?;
    let registers = map_registers(&device)?;
    let mut controller = Controller::new(Registers(registers), Monotonic, TRANSFER_NANOS);
    controller.disable();
    controller.clear_interrupts();
    controller.enable_master();

    let mut touchpad = describe(&mut controller)?;
    say(&touchpad);
    let port = port::create(Kernel).map_err(|_| Step::Events)?;
    publish(&touchpad, &start, &control, &port)?;
    serve(&mut controller, &mut touchpad, &control)
}

/// The HELLO for the touchpad, at `location`.
fn hello(touchpad: &Touchpad, location: u32) -> Hello {
    let descriptor = &touchpad.descriptor;
    let mut hello = Hello::EMPTY;
    hello.version = message::VERSION;
    hello.location = location;
    hello.id = DeviceId {
        bustype: BUS_I2C,
        vendor: descriptor.vendor_id,
        product: descriptor.product_id,
        version: descriptor.version_id,
    };
    hello.name = hid::name(&[], &[], b"I2C HID Touchpad", touchpad.interpreter.kind());
    hello.bits = touchpad.interpreter.bits();
    hello.axes = touchpad.interpreter.axes();
    hello
}

/// Tell the core what the touchpad offers, and wait for its answer.
fn publish(
    touchpad: &Touchpad,
    start: &Start,
    control: &Channel<Kernel>,
    port: &Port<Kernel>,
) -> Result<(), Step> {
    let share = port
        .as_owned()
        .duplicate(Requested::Exactly(message::PORT_RIGHTS))
        .map_err(|_| Step::Hello)?;
    control
        .write_with(Message::Hello(hello(touchpad, start.location)).encode().as_bytes(), [share])
        .map_err(|_| Step::Hello)?;
    let _ = control
        .wait_one(Signals::READABLE, Deadline::Never)
        .map_err(|_| Step::Hello)?;
    let mut bytes = [0_u8; message::MAX_BYTES];
    let mut handles = [Handle::INVALID; 1];
    let received = control.read(&mut bytes, &mut handles).map_err(|_| Step::Hello)?;
    for handle in handles.iter().take(received.handles) {
        drop(OwnedHandle::from_raw(Kernel, *handle));
    }
    let decoded =
        Message::decode(bytes.get(..received.bytes).unwrap_or_default()).map_err(|_| Step::Control)?;
    match decoded {
        Message::Ready(_) => Ok(()),
        _ => Err(Step::Control),
    }
}

/// Poll the touchpad, and hand the core what it says.
fn serve<M: Memory, C: Clock>(
    controller: &mut Controller<M, C>,
    touchpad: &mut Touchpad,
    control: &Channel<Kernel>,
) -> Result<(), Step> {
    let mut events = EventBuf::default();
    let mut previous = [0_u8; MAX_INPUT_REPORT];
    let mut current = [0_u8; MAX_INPUT_REPORT];
    loop {
        if tick(control)? {
            return Ok(());
        }
        let length = usize::from(touchpad.descriptor.max_input_length).min(MAX_INPUT_REPORT);
        let Some(slice) = current.get_mut(..length) else {
            return Err(Step::Device);
        };
        slice.fill(0);
        if read_register(controller, touchpad.descriptor.input_register, slice).is_err() {
            // The bus did not answer: the controller is put back to what it
            // was, and the next poll tries again.
            controller.disable();
            controller.clear_interrupts();
            controller.enable_master();
            continue;
        }
        if previous.get(..length) == Some(&*slice) {
            continue;
        }
        if let Some(slot) = previous.get_mut(..length) {
            slot.copy_from_slice(slice);
        }
        events.clear();
        touchpad.interpreter.report(slice, &mut events);
        if events.as_slice().is_empty() {
            continue;
        }
        let Some(batch) = Events::new(events.as_slice()) else {
            continue;
        };
        control
            .write(Message::Events(batch).encode().as_bytes())
            .map_err(|_| Step::Control)?;
    }
}

/// Wait for the next poll, and say whether the core asked the driver to
/// stop.
fn tick(control: &Channel<Kernel>) -> Result<bool, Step> {
    let now = xaqu_rt::linux::monotonic_nanos().unwrap_or(0);
    let deadline = now.saturating_add(POLL_NANOS);
    match control.wait_one(
        Signals::READABLE | Signals::PEER_CLOSED,
        Deadline::At(deadline),
    ) {
        Ok(_) => {}
        Err(RtError::TimedOut) => return Ok(false),
        Err(_) => return Err(Step::Control),
    }
    let mut bytes = [0_u8; message::MAX_BYTES];
    let mut handles = [Handle::INVALID; 1];
    let received = match control.read(&mut bytes, &mut handles) {
        Ok(received) => received,
        Err(ReadError::Failed(RtError::PeerClosed)) => return Ok(true),
        Err(ReadError::Failed(RtError::ShouldWait)) => return Ok(false),
        Err(_) => return Err(Step::Control),
    };
    for handle in handles.iter().take(received.handles) {
        drop(OwnedHandle::from_raw(Kernel, *handle));
    }
    let decoded =
        Message::decode(bytes.get(..received.bytes).unwrap_or_default()).map_err(|_| Step::Control)?;
    match decoded {
        Message::Stop => {
            control
                .write(Message::Stopped.encode().as_bytes())
                .map_err(|_| Step::Control)?;
            Ok(true)
        }
        Message::Refused(_) => Err(Step::Control),
        _ => Ok(false),
    }
}
