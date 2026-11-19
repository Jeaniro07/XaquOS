//! The Intel Serial I/O (LPSS) I2C host controller, as logic over registers.
//!
//! An I2C controller is a small state machine: a target address, a command
//! register that says "this byte is data, or this byte is to be read", and
//! two flags -- "there is room in the transmit FIFO" and "there is a byte in
//! the receive FIFO" -- that say whether the controller may be written to
//! again. A driver that does not watch those flags overruns the controller's
//! FIFO; one that does not watch the abort flag waits forever for a device
//! that has already said no.
//!
//! This crate is that state machine, and nothing else: it does not know what
//! bus the controller is on, what device answers on it, or what the bytes
//! mean. It is written over two traits -- [`controller::Memory`] for the
//! registers and [`controller::Clock`] for the time -- so that it can be
//! tested on the host against a register file, and driven in a native driver
//! against the controller's own registers.
//!
//! # Where the register map comes from
//!
//! The Intel Serial I/O (LPSS) I2C controller's register block is the
//! Synopsys DesignWare I2C core's, as the Intel datasheets describe it: the
//! "Serial I/O (LPSS)" chapter of the processor datasheet, section
//! "I2C Host Controller Register Details", gives `IC_CON` at 0x000, `IC_TAR`
//! at 0x004, `IC_DATA_CMD` at 0x010 and `IC_STATUS` at 0x070. Linux drives
//! the same controllers with its DesignWare driver
//! (`drivers/i2c/busses/i2c-designware-core.h`, whose offsets are the ones
//! here), because Linux's LPSS support creates an "i2c_designware" device
//! for every LPSS I2C controller (`drivers/mfd/intel-lpss.c`).
//!
//! # The bus speed
//!
//! The controller has no speed field: the DesignWare core's own `IC_CON`
//! carries one, but the LPSS part has none, and the divider that decides the
//! bus clock lives in the platform's private registers beside the
//! controller, not in the controller's own block (Linux touches those
//! private registers for the LPSS clock dividers in
//! `drivers/acpi/x86/lpss.c`). So this crate takes the controller's input
//! clock as a parameter and does not invent a divider: a driver that wants
//! a bus speed must program the platform's divider, which is the upgrade
//! path, and this crate is where the input clock would be used.

#![no_std]
#![forbid(unsafe_code)]

pub mod controller;
pub mod regs;

#[cfg(test)]
mod tests;
