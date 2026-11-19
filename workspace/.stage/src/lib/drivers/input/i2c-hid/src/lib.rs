//! The HID over I2C protocol: what a HID device on an I2C bus keeps, and how
//! it is talked to.
//!
//! A HID device on I2C does not answer at a fixed register. It keeps a *HID
//! descriptor* at a register of its own, and that descriptor says where every
//! other register is: where the report descriptor lies, where an input
//! report is read from, where a command is written, and how long each may
//! be. A driver that does not read the descriptor first is guessing, and the
//! guess is wrong on any device that does not use the default registers.
//!
//! The descriptor is thirty bytes, little-endian, as Linux's
//! `drivers/hid/i2c-hid.c` reads it (`struct i2c_hid_desc`). The commands
//! are four bytes: a register, a report type and identifier, and an opcode.
//! A command that carries data is written after the four bytes, in one
//! transfer; a command that only asks is written alone and the answer is
//! read back in a second transfer.
//!
//! This crate is the protocol alone: it says what the bytes are. It does not
//! drive a bus, does not schedule, and does not parse reports into events:
//! the report descriptor's meaning is the HID layer's business, and this
//! repository already has one, in the USB host driver.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;

mod command;
mod descriptor;

pub use command::{Command, Opcode, ReportType};
pub use descriptor::{DESCRIPTOR_BYTES, Descriptor, Malformed};
