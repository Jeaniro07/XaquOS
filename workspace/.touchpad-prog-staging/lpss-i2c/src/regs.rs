//! The Intel Serial I/O (LPSS) I2C host controller's registers and their bits.
//!
//! Every value here is the controller's, as the Intel "Serial I/O (LPSS)"
//! chapter of the processor datasheet gives it ("I2C Host Controller
//! Register Details"), for the offsets, and the DesignWare I2C core's
//! layout, as Linux's `drivers/i2c/busses/i2c-designware-core.h` names the
//! bits, for the meanings. Where a register is one this crate needs but the
//! datasheet's summary does not name, the module says so.
//!
//! The register block is 4 KiB and every register in it is 32 bits wide.

/// The control register: what the controller is doing, and whether it is
/// doing it at all.
///
/// Bit 0 enables the controller; a write to this register only takes effect
/// while the controller is disabled.
pub const IC_CON: u32 = 0x000;

/// The target address register: which device on the bus this transaction is
/// for.
///
/// The address is in the low ten bits; the rest of the register is for the
/// general call and the special commands, which a driver of a HID device
/// never sends.
pub const IC_TAR: u32 = 0x004;

/// The data and command register: the byte to send, or the byte to read.
///
/// Writing it sends a byte; reading it takes the byte the device answered
/// with. The command bits say which of the two, and whether a stop or a
/// repeated start follows.
pub const IC_DATA_CMD: u32 = 0x010;

/// The data register: the byte the device answered with.
pub const IC_DATA: u32 = 0x014;

/// The interrupt status register: the interrupts the controller has raised
/// and not been told to forget. Its bits are written back to clear them.
pub const IC_INTR_STAT: u32 = 0x02C;

/// The raw interrupt status register: the same interrupts, before the
/// controller's masks are applied. A driver that wants to know what the
/// controller saw reads this one.
pub const IC_RAW_INTR_STAT: u32 = 0x034;

/// The receive FIFO threshold: how many bytes in the receive FIFO make the
/// controller raise an interrupt. A polled driver leaves it as it is.
pub const IC_RX_TL: u32 = 0x038;

/// The status register: what the controller may be asked to do next.
pub const IC_STATUS: u32 = 0x070;

/// The transmit abort source register: why the controller gave up.
///
/// This register is the DesignWare core's, at the offset Linux's
/// `i2c-designware-core.h` gives (`DW_IC_TX_ABRT_SOURCE`, 0x80). That the
/// LPSS part has it is not verified, so a driver that reads zero here -- a
/// part without the register, or a read that does not reach it -- is told
/// the transfer was aborted rather than that the device nacked it, which is
/// the safe reading either way.
pub const IC_TX_ABRT_SOURCE: u32 = 0x080;

/// `IC_CON.ENABLE`: the controller is on.
pub const CON_ENABLE: u32 = 1 << 0;

/// `IC_CON.RESTART`: the controller may send a repeated start.
pub const CON_RESTART: u32 = 1 << 1;

/// `IC_CON.MASTER`: the controller is the master on the bus.
pub const CON_MASTER: u32 = 1 << 2;

/// `IC_CON.10BITADDR`: the target address is ten bits wide.
pub const CON_10BITADDR: u32 = 1 << 3;

/// `IC_CON.STRETCH`: the controller stretches the clock rather than
/// answering with a nack when the bus is held.
pub const CON_STRETCH: u32 = 1 << 4;

/// `IC_TAR.TARGET_ADDR`: the address of the device this transaction is for.
pub const TAR_TARGET_ADDR: u32 = 0x03FF;

/// `IC_DATA_CMD.CMD`: the byte is to be read, not written. A read.
pub const DATA_CMD_CMD: u32 = 1 << 0;

/// `IC_DATA_CMD.ST`: a stop follows this byte: the transaction ends here.
pub const DATA_CMD_ST: u32 = 1 << 8;

/// `IC_DATA_CMD.RESTART`: a repeated start precedes this byte: the
/// transaction continues without releasing the bus.
pub const DATA_CMD_RESTART: u32 = 1 << 9;

/// `IC_DATA.DAT`: the byte itself.
pub const DATA_DAT: u32 = 0x00FF;

/// `IC_INTR_STAT.TX_UNDER`: the transmit FIFO ran empty mid-byte.
pub const INTR_TX_UNDER: u32 = 1 << 0;

/// `IC_INTR_STAT.RX_OVER`: the receive FIFO overflowed: bytes arrived
/// faster than they were read.
pub const INTR_RX_OVER: u32 = 1 << 1;

/// `IC_INTR_STAT.RX_FULL`: the receive FIFO holds at least the threshold.
pub const INTR_RX_FULL: u32 = 1 << 2;

/// `IC_INTR_STAT.TX_EMPTY`: the transmit FIFO has emptied.
pub const INTR_TX_EMPTY: u32 = 1 << 3;

/// `IC_INTR_STAT.RD_REQ`: a device on the bus asked the controller to read.
pub const INTR_RD_REQ: u32 = 1 << 4;

/// `IC_INTR_STAT.TX_ABORT`: the controller gave up on the transfer.
pub const INTR_TX_ABORT: u32 = 1 << 5;

/// `IC_INTR_STAT.RX_DONE`: a receive is finished.
pub const INTR_RX_DONE: u32 = 1 << 6;

/// `IC_INTR_STAT.ACTIVITY`: the controller is busy on the bus.
pub const INTR_ACTIVITY: u32 = 1 << 7;

/// `IC_STATUS.ACTIVITY`: the controller is busy on the bus.
pub const STATUS_ACTIVITY: u32 = 1 << 0;

/// `IC_STATUS.TFNF`: the transmit FIFO has room: another byte may be written.
pub const STATUS_TFNF: u32 = 1 << 1;

/// `IC_STATUS.TFE`: the transmit FIFO is empty: everything written has gone.
pub const STATUS_TFE: u32 = 1 << 2;

/// `IC_STATUS.RFNE`: the receive FIFO holds a byte: it may be read.
pub const STATUS_RFNE: u32 = 1 << 3;

/// `IC_STATUS.MST_ACTIVITY`: the controller is the master and is busy.
pub const STATUS_MST_ACTIVITY: u32 = 1 << 4;

/// `IC_STATUS.SLV_ACTIVITY`: the controller is a slave and is busy.
pub const STATUS_SLV_ACTIVITY: u32 = 1 << 5;

/// `IC_TX_ABRT_SOURCE.ABRT_7B_ADDR_NOACK`: the device did not answer to its
/// address: it is not there, or it is not listening.
pub const ABRT_7B_ADDR_NOACK: u32 = 1 << 0;
