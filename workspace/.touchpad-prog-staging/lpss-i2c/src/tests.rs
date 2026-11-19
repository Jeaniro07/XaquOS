//! The tests of the LPSS I2C controller: a register file that behaves like
//! the controller, and a clock that only the test controls.

extern crate std;

use core::cell::Cell;
use std::rc::Rc;
use std::vec::Vec;

use super::controller::{Clock, Controller, Error, Memory};
use super::regs;

/// How long a test's waits may take, in nanoseconds.
const TIMEOUT: u64 = 10_000;

/// The address the tests talk to.
const ADDRESS: u8 = 0x5D;

/// The registers, and what the controller has said to them.
struct Fake {
    /// The registers, as the controller wrote them.
    registers: [u32; 64],
    /// The bytes the device answers a read with.
    answers: Vec<u8>,
    /// The byte a read command fetched, waiting to be read from the data
    /// register.
    pending: Option<u8>,
    /// Every command written to the data and command register.
    commands: Vec<u32>,
    /// Whether the controller may send: with it clear, the transmit FIFO
    /// never has room and every write waits for it.
    ready: bool,
    /// The interrupts the controller has raised.
    raw: u32,
    /// Why the controller aborted.
    abort_source: u32,
}

impl Fake {
    /// A controller that may send and receive, and a device that answers
    /// with `answers`.
    fn new(answers: &[u8]) -> Self {
        Self {
            registers: [0; 64],
            answers: answers.to_vec(),
            pending: None,
            commands: Vec::new(),
            ready: true,
            raw: 0,
            abort_source: 0,
        }
    }

    /// The register at `offset`, as the controller last wrote it.
    fn register(&self, offset: u32) -> u32 {
        *self.registers.get((offset / 4) as usize).unwrap_or(&0)
    }
}

impl Memory for Fake {
    fn read_u32(&self, offset: u32) -> u32 {
        match offset {
            regs::IC_STATUS => {
                let mut status = 0;
                if self.ready {
                    status |= regs::STATUS_TFNF | regs::STATUS_TFE;
                }
                if self.pending.is_some() {
                    status |= regs::STATUS_RFNE;
                }
                status
            }
            regs::IC_DATA => self.pending.unwrap_or(0),
            regs::IC_RAW_INTR_STAT => self.raw,
            _ => self.register(offset),
        }
    }

    fn write_u32(&mut self, offset: u32, value: u32) {
        match offset {
            regs::IC_DATA_CMD => {
                self.commands.push(value);
                if value & regs::DATA_CMD_CMD != 0 {
                    let first = self.answers.first().copied();
                    if let Some(byte) = first {
                        self.pending = Some(byte);
                        self.answers.remove(0);
                    }
                }
            }
            regs::IC_INTR_STAT => self.raw &= !value,
            _ => {
                if let Some(slot) = self.registers.get_mut((offset / 4) as usize) {
                    *slot = value;
                }
            }
        }
    }
}

/// Memory over a borrow of the register file, so a test may look at what
/// the controller said once it is done with it.
impl Memory for &mut Fake {
    fn read_u32(&self, offset: u32) -> u32 {
        (**self).read_u32(offset)
    }

    fn write_u32(&mut self, offset: u32, value: u32) {
        (**self).write_u32(offset, value);
    }
}

/// A clock that advances a fixed step every time it is asked, so a wait
/// ends rather than spinning.
struct FakeClock {
    /// The time, shared with the test.
    nanos: Rc<Cell<u64>>,
    /// How far each reading moves the time.
    step: u64,
}

impl FakeClock {
    /// A clock, and the time it keeps.
    fn new() -> (FakeClock, Rc<Cell<u64>>) {
        let nanos = Rc::new(Cell::new(0));
        let clock = FakeClock {
            nanos: Rc::clone(&nanos),
            step: 1_000,
        };
        (clock, nanos)
    }
}

impl Clock for FakeClock {
    fn now(&mut self) -> u64 {
        let now = self.nanos.get() + self.step;
        self.nanos.set(now);
        now
    }
}

/// The controller over a register file, and the file itself.
fn controller(fake: &mut Fake) -> Controller<&mut Fake, FakeClock> {
    let (clock, _nanos) = FakeClock::new();
    Controller::new(&mut *fake, clock, TIMEOUT)
}

#[test]
fn disable_clears_the_enable_bit() {
    let mut fake = Fake::new(&[]);
    {
        let mut controller = controller(&mut fake);
        controller.enable_master();
        controller.disable();
    }
    let control = fake.register(regs::IC_CON);
    assert_eq!(
        control & regs::CON_ENABLE,
        0,
        "the controller should be off after a disable"
    );
}

#[test]
fn enable_master_sets_enable_and_master() {
    let mut fake = Fake::new(&[]);
    {
        let mut controller = controller(&mut fake);
        controller.enable_master();
    }
    let control = fake.register(regs::IC_CON);
    assert_ne!(
        control & regs::CON_ENABLE,
        0,
        "the controller should be on after an enable"
    );
    assert_ne!(
        control & regs::CON_MASTER,
        0,
        "the controller should be the master of the bus"
    );
}

#[test]
fn enable_master_clears_the_ten_bit_address() {
    let mut fake = Fake::new(&[]);
    {
        let mut controller = controller(&mut fake);
        controller.enable_master();
    }
    let control = fake.register(regs::IC_CON);
    assert_eq!(
        control & regs::CON_10BITADDR,
        0,
        "a HID device answers to a seven-bit address"
    );
}

#[test]
fn set_target_puts_the_address_in_the_low_ten_bits() {
    let mut fake = Fake::new(&[]);
    {
        let mut controller = controller(&mut fake);
        controller.set_target(ADDRESS);
    }
    assert_eq!(
        fake.register(regs::IC_TAR),
        u32::from(ADDRESS),
        "the target address should be the device's address"
    );
}

#[test]
fn set_target_masks_the_address_to_ten_bits() {
    let mut fake = Fake::new(&[]);
    {
        let mut controller = controller(&mut fake);
        controller.set_target(0xFF);
    }
    assert_eq!(
        fake.register(regs::IC_TAR) & !regs::TAR_TARGET_ADDR,
        0,
        "nothing but the address belongs in the target register"
    );
}

#[test]
fn writes_the_bytes_in_order() {
    let mut fake = Fake::new(&[]);
    {
        let mut controller = controller(&mut fake);
        let result = controller.write_bytes(ADDRESS, &[0x01, 0x02, 0x03], true);
        assert_eq!(result, Ok(()), "the write should finish");
    }
    let expected: &[u32] = &[0x01, 0x02, 0x03 | regs::DATA_CMD_ST];
    assert_eq!(
        fake.commands.as_slice(),
        expected,
        "the bytes should go out in the order they were given"
    );
}

#[test]
fn the_stop_bit_is_on_the_last_byte_only() {
    let mut fake = Fake::new(&[]);
    {
        let mut controller = controller(&mut fake);
        let _ = controller.write_bytes(ADDRESS, &[0x01, 0x02], true);
    }
    let first = fake.commands.first().copied().unwrap_or(0);
    let last = fake.commands.last().copied().unwrap_or(0);
    assert_eq!(
        first & regs::DATA_CMD_ST,
        0,
        "only the last byte of a transaction ends it"
    );
    assert_ne!(
        last & regs::DATA_CMD_ST,
        0,
        "the last byte of a transaction ends it"
    );
}

#[test]
fn a_write_without_a_stop_leaves_the_bus_busy() {
    let mut fake = Fake::new(&[]);
    {
        let mut controller = controller(&mut fake);
        let _ = controller.write_bytes(ADDRESS, &[0x01], false);
        assert!(
            controller.busy(),
            "the bus should still be held after a write without a stop"
        );
    }
    let command = fake.commands.last().copied().unwrap_or(0);
    assert_eq!(
        command & regs::DATA_CMD_ST,
        0,
        "a write without a stop must not end the transaction"
    );
}

#[test]
fn a_read_takes_the_bytes_the_device_answers_with() {
    let mut fake = Fake::new(&[0xAA, 0xBB]);
    let mut out = [0_u8; 2];
    {
        let mut controller = controller(&mut fake);
        let result = controller.read_bytes(ADDRESS, &mut out, true);
        assert_eq!(result, Ok(()), "the read should finish");
    }
    assert_eq!(
        out,
        [0xAA, 0xBB],
        "the bytes should be the ones the device answered with"
    );
}

#[test]
fn the_stop_bit_is_on_the_last_read_command() {
    let mut fake = Fake::new(&[0x01, 0x02]);
    let mut out = [0_u8; 2];
    {
        let mut controller = controller(&mut fake);
        let _ = controller.read_bytes(ADDRESS, &mut out, true);
    }
    let last = fake.commands.last().copied().unwrap_or(0);
    assert_ne!(
        last & regs::DATA_CMD_ST,
        0,
        "the last byte read ends the transaction"
    );
    assert_ne!(
        last & regs::DATA_CMD_CMD,
        0,
        "a read command says so"
    );
}

#[test]
fn a_read_after_a_write_without_a_stop_is_a_repeated_start() {
    let mut fake = Fake::new(&[0x11]);
    let mut out = [0_u8; 1];
    {
        let mut controller = controller(&mut fake);
        let _ = controller.write_bytes(ADDRESS, &[0x22], false);
        let _ = controller.read_bytes(ADDRESS, &mut out, true);
    }
    let command = fake.commands.last().copied().unwrap_or(0);
    assert_ne!(
        command & regs::DATA_CMD_RESTART,
        0,
        "a read that follows a write without a stop repeats the start"
    );
}

#[test]
fn a_read_after_a_write_with_a_stop_is_not_a_repeated_start() {
    let mut fake = Fake::new(&[0x11]);
    let mut out = [0_u8; 1];
    {
        let mut controller = controller(&mut fake);
        let _ = controller.write_bytes(ADDRESS, &[0x22], true);
        let _ = controller.read_bytes(ADDRESS, &mut out, true);
    }
    let command = fake.commands.last().copied().unwrap_or(0);
    assert_eq!(
        command & regs::DATA_CMD_RESTART,
        0,
        "a read that follows a stop is a new transaction"
    );
}

#[test]
fn a_transfer_that_never_starts_times_out() {
    let mut fake = Fake::new(&[]);
    fake.ready = false;
    {
        let mut controller = controller(&mut fake);
        let result = controller.write_bytes(ADDRESS, &[0x01], true);
        assert_eq!(result, Err(Error::Timeout), "the write should time out");
    }
}

#[test]
fn an_abort_is_reported() {
    let mut fake = Fake::new(&[]);
    fake.raw = regs::INTR_TX_ABORT;
    {
        let mut controller = controller(&mut fake);
        let result = controller.write_bytes(ADDRESS, &[0x01], true);
        assert_eq!(result, Err(Error::Aborted), "the abort should be reported");
    }
}

#[test]
fn a_nack_is_reported() {
    let mut fake = Fake::new(&[]);
    fake.raw = regs::INTR_TX_ABORT;
    fake.abort_source = regs::ABRT_7B_ADDR_NOACK;
    {
        let mut controller = controller(&mut fake);
        let result = controller.write_bytes(ADDRESS, &[0x01], true);
        assert_eq!(result, Err(Error::Nack), "the nack should be reported");
    }
}

#[test]
fn an_overrun_is_reported() {
    let mut fake = Fake::new(&[]);
    fake.raw = regs::INTR_RX_OVER;
    {
        let mut controller = controller(&mut fake);
        let result = controller.write_bytes(ADDRESS, &[0x01], true);
        assert_eq!(result, Err(Error::Overrun), "the overrun should be reported");
    }
}

#[test]
fn an_underrun_is_reported() {
    let mut fake = Fake::new(&[]);
    fake.raw = regs::INTR_TX_UNDER;
    {
        let mut controller = controller(&mut fake);
        let result = controller.write_bytes(ADDRESS, &[0x01], true);
        assert_eq!(
            result,
            Err(Error::Underrun),
            "the underrun should be reported"
        );
    }
}

#[test]
fn clearing_the_interrupts_writes_the_raw_status_back() {
    let mut fake = Fake::new(&[]);
    fake.raw = regs::INTR_RX_FULL;
    {
        let mut controller = controller(&mut fake);
        controller.clear_interrupts();
    }
    assert_eq!(
        fake.raw & regs::INTR_RX_FULL,
        0,
        "the interrupt should be forgotten once it is cleared"
    );
}

#[test]
fn a_timeout_is_measured_in_nanoseconds() {
    let mut fake = Fake::new(&[]);
    fake.ready = false;
    let (clock, nanos) = FakeClock::new();
    let mut controller = Controller::new(&mut fake, clock, TIMEOUT);
    let result = controller.write_bytes(ADDRESS, &[0x01], true);
    assert_eq!(result, Err(Error::Timeout), "the write should time out");
    assert!(
        nanos.get() >= TIMEOUT,
        "the wait should have lasted at least the timeout"
    );
}
