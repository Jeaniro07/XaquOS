//! Transactions on an Intel Serial I/O (LPSS) I2C controller.
//!
//! The controller is driven by writing its registers and reading them back,
//! and the only thing that makes a transaction is: set the target address,
//! write the bytes to `IC_DATA_CMD`, and watch two flags -- "there is room
//! in the transmit FIFO" and "there is a byte in the receive FIFO" -- so
//! that the controller is never written faster than it can take, and never
//! read before it has answered. A third flag, the abort, ends a transfer
//! early: the device did not answer, or the bus did not behave, and the
//! driver must not wait for bytes that will never come.
//!
//! Every wait here is bounded: a controller that never raises the flag it
//! was waited for ends the wait with [`Error::Timeout`] rather than hanging
//! the driver, which on a machine with no other input would be a machine
//! that cannot be typed at.
//!
//! The state machine knows nothing about what is on the bus. A HID device
//! on an I2C bus keeps its registers elsewhere -- in its own descriptor --
//! and reading one of them is a write of the register number followed by a
//! read of the bytes, which is what [`Controller::write_bytes`] and
//! [`Controller::read_bytes`] are for: the write leaves the bus busy (no
//! stop), so the read that follows is a repeated start.

use crate::regs;

/// The registers of a controller, read and written by offset.
///
/// A native driver implements this over the controller's mapped register
/// block; a test implements it over a register file.
pub trait Memory {
    /// The register at `offset`, as it is read.
    fn read_u32(&self, offset: u32) -> u32;

    /// Write `value` to the register at `offset`.
    fn write_u32(&mut self, offset: u32, value: u32);
}

/// A clock, in nanoseconds.
///
/// The clock is taken by `&mut self` so that a test's clock may advance as
/// it is asked, which is what makes a timeout testable at all.
pub trait Clock {
    /// The time now, in nanoseconds since whatever the platform counts from.
    fn now(&mut self) -> u64;
}

/// Why a transaction did not finish.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    /// The controller did not raise the flag the transfer waited for, in the
    /// time it was given.
    Timeout,
    /// The controller gave up on the transfer.
    Aborted,
    /// The device did not answer to its address.
    Nack,
    /// The receive FIFO overflowed: bytes arrived faster than they were
    /// read.
    Overrun,
    /// The receive FIFO ran empty mid-transfer.
    Underrun,
}

/// A controller, driven over its registers.
///
/// The controller holds the registers and the clock, and remembers whether
/// the bus is busy, so that a read which follows a write is a repeated
/// start rather than a new transaction.
#[derive(Debug)]
pub struct Controller<M, C> {
    /// The registers.
    memory: M,
    /// The clock the timeouts are measured on.
    clock: C,
    /// How long a wait for a flag may take, in nanoseconds.
    timeout_nanos: u64,
    /// Whether a transaction is on the bus without a stop: the next read
    /// is a repeated start.
    busy: bool,
}

impl<M: Memory, C: Clock> Controller<M, C> {
    /// A controller over `memory`, timed by `clock`, where a wait for a flag
    /// may take `timeout_nanos`.
    #[must_use]
    pub const fn new(memory: M, clock: C, timeout_nanos: u64) -> Self {
        Self {
            memory,
            clock,
            timeout_nanos,
            busy: false,
        }
    }

    /// The registers, as they are read.
    #[must_use]
    pub const fn memory(&self) -> &M {
        &self.memory
    }

    /// The registers, as they are written.
    #[must_use]
    pub fn memory_mut(&mut self) -> &mut M {
        &mut self.memory
    }

    /// Whether a transaction is on the bus: a read that follows is a
    /// repeated start.
    #[must_use]
    pub const fn busy(&self) -> bool {
        self.busy
    }

    /// Turn the controller off.
    ///
    /// The controller must be disabled before its control register is
    /// written, so this is what comes first when the controller is brought
    /// up.
    pub fn disable(&mut self) {
        let control = self.memory.read_u32(regs::IC_CON);
        self.memory
            .write_u32(regs::IC_CON, control & !regs::CON_ENABLE);
        self.busy = false;
    }

    /// Turn the controller on, as the master of the bus.
    ///
    /// The address is seven bits wide: a ten-bit address is a bus that
    /// almost nothing speaks, and the controller's own register says so.
    pub fn enable_master(&mut self) {
        let control = self.memory.read_u32(regs::IC_CON);
        let control = (control | regs::CON_ENABLE | regs::CON_MASTER) & !regs::CON_10BITADDR;
        self.memory.write_u32(regs::IC_CON, control);
    }

    /// Forget the interrupts the controller has raised.
    ///
    /// The interrupt status register is written back with what it holds,
    /// which is how the controller's interrupts are cleared.
    pub fn clear_interrupts(&mut self) {
        let raw = self.memory.read_u32(regs::IC_RAW_INTR_STAT);
        self.memory.write_u32(regs::IC_INTR_STAT, raw);
    }

    /// Point the controller at the device at `address`.
    pub fn set_target(&mut self, address: u8) {
        let target = u32::from(address) & regs::TAR_TARGET_ADDR;
        self.memory.write_u32(regs::IC_TAR, target);
    }

    /// Write `bytes` to the device at `address`.
    ///
    /// With `stop`, the last byte ends the transaction and the bus is
    /// released. Without it, the bus is held for the transaction that
    /// follows, which is how a register number is written and then read
    /// from without the device in between seeing a stop.
    ///
    /// # Errors
    ///
    /// [`Error::Timeout`] when the controller will not take the bytes, and
    /// [`Error::Aborted`], [`Error::Nack`], [`Error::Overrun`] or
    /// [`Error::Underrun`] when the controller says the transfer failed.
    pub fn write_bytes(&mut self, address: u8, bytes: &[u8], stop: bool) -> Result<(), Error> {
        self.set_target(address);
        let last = bytes.len().saturating_sub(1);
        for (index, byte) in bytes.iter().enumerate() {
            self.wait_for(regs::STATUS_TFNF)?;
            self.check_failed()?;
            let mut command = u32::from(*byte);
            if index == last && stop {
                command |= regs::DATA_CMD_ST;
            }
            self.memory.write_u32(regs::IC_DATA_CMD, command);
        }
        self.busy = !stop;
        if bytes.is_empty() {
            return Ok(());
        }
        self.wait_for(regs::STATUS_TFE)?;
        self.check_failed()?;
        Ok(())
    }

    /// Read up to `out.len()` bytes from the device at `address`.
    ///
    /// With `stop`, the last byte ends the transaction. A read that follows
    /// a write without a stop is a repeated start, which the controller is
    /// told of by the command itself.
    ///
    /// # Errors
    ///
    /// [`Error::Timeout`] when the device does not answer in time, and
    /// [`Error::Aborted`], [`Error::Nack`], [`Error::Overrun`] or
    /// [`Error::Underrun`] when the controller says the transfer failed.
    pub fn read_bytes(
        &mut self,
        address: u8,
        out: &mut [u8],
        stop: bool,
    ) -> Result<(), Error> {
        self.set_target(address);
        let last = out.len().saturating_sub(1);
        let restart = self.busy;
        for (index, slot) in out.iter_mut().enumerate() {
            self.wait_for(regs::STATUS_TFNF)?;
            self.check_failed()?;
            let mut command = regs::DATA_CMD_CMD;
            if index == 0 && restart {
                command |= regs::DATA_CMD_RESTART;
            }
            if index == last && stop {
                command |= regs::DATA_CMD_ST;
            }
            self.memory.write_u32(regs::IC_DATA_CMD, command);
            self.wait_for(regs::STATUS_RFNE)?;
            self.check_failed()?;
            let data = self.memory.read_u32(regs::IC_DATA) & regs::DATA_DAT;
            *slot = data as u8;
        }
        self.busy = !stop;
        Ok(())
    }

    /// Wait until the status register holds one of `mask`'s bits.
    ///
    /// # Errors
    ///
    /// [`Error::Timeout`] when the time runs out first.
    fn wait_for(&mut self, mask: u32) -> Result<u32, Error> {
        let deadline = self.clock.now().saturating_add(self.timeout_nanos);
        loop {
            let status = self.memory.read_u32(regs::IC_STATUS);
            if status & mask != 0 {
                return Ok(status);
            }
            if self.clock.now() >= deadline {
                return Err(Error::Timeout);
            }
        }
    }

    /// Say whether the controller has given up on the transfer.
    ///
    /// # Errors
    ///
    /// The [`Error`] the controller's own status names.
    fn check_failed(&mut self) -> Result<(), Error> {
        let raw = self.memory.read_u32(regs::IC_RAW_INTR_STAT);
        if raw & regs::INTR_TX_ABORT != 0 {
            let source = self.memory.read_u32(regs::IC_TX_ABRT_SOURCE);
            if source & regs::ABRT_7B_ADDR_NOACK != 0 {
                return Err(Error::Nack);
            }
            return Err(Error::Aborted);
        }
        if raw & regs::INTR_RX_OVER != 0 {
            return Err(Error::Overrun);
        }
        if raw & regs::INTR_TX_UNDER != 0 {
            return Err(Error::Underrun);
        }
        Ok(())
    }
}
