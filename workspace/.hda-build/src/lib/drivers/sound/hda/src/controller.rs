//! The controller's register block, and the few operations over it that are
//! not a verb.
//!
//! The block is a window of the controller's memory, read and written one
//! byte, two bytes or four bytes at a time. [`Mmio`] is that window; the
//! driver program maps the controller's BAR and implements it over volatile
//! accesses, and the host tests implement it over an array.
//!
//! The offsets and the bits are those of the Intel High Definition Audio
//! specification's chapter 3, as QEMU's `intel-hda` and Linux's
//! `sound/pci/hda` read them; `docs/HDA.md` §2 cites them.

/// The controller's registers, and where each sits in its register block.
pub mod register {
    /// The capabilities register: what the controller has.
    pub const GCAP: u32 = 0x00;
    /// The global control register.
    pub const GCTL: u32 = 0x08;
    /// The wake-enable register: which codec's state wakes the machine.
    pub const WAKEEN: u32 = 0x0C;
    /// The global status register: which codecs are out of their reset.
    pub const STATESTS: u32 = 0x0E;
    /// The interrupt control register.
    pub const INTCTL: u32 = 0x20;
    /// The interrupt status register.
    pub const INTSTS: u32 = 0x24;
    /// The wall clock, counting at 24 MHz.
    pub const WALLCLK: u32 = 0x30;
    /// The CORB's base address, in the lower half.
    pub const CORBLBASE: u32 = 0x40;
    /// The CORB's base address, in the upper half.
    pub const CORBUBASE: u32 = 0x44;
    /// The CORB's write pointer: the entry the next verb goes into.
    pub const CORBWP: u32 = 0x48;
    /// The CORB's read pointer: the entry the controller has taken.
    pub const CORBRP: u32 = 0x4A;
    /// The CORB's control register.
    pub const CORBCTL: u32 = 0x4C;
    /// The CORB's status register.
    pub const CORBSTS: u32 = 0x4D;
    /// The CORB's size.
    pub const CORBSIZE: u32 = 0x4E;
    /// The RIRB's base address, in the lower half.
    pub const RIRBLBASE: u32 = 0x50;
    /// The RIRB's base address, in the upper half.
    pub const RIRBUBASE: u32 = 0x54;
    /// The RIRB's write pointer: the entry the last answer went into.
    pub const RIRBWP: u32 = 0x58;
    /// The interrupt count: how many answers the controller may give.
    pub const RINTCNT: u32 = 0x5A;
    /// The RIRB's control register.
    pub const RIRBCTL: u32 = 0x5C;
    /// The RIRB's status register.
    pub const RIRBSTS: u32 = 0x5D;
    /// The RIRB's size.
    pub const RIRBSIZE: u32 = 0x5E;
    /// The first stream's register block.
    pub const STREAMS: u32 = 0x80;
    /// The distance between two streams' register blocks.
    pub const STREAM_STRIDE: u32 = 0x20;
    /// The first output stream's register block.
    pub const OUTPUT_STREAMS: u32 = 0x100;
}

/// The registers of one stream, within its own block.
pub mod stream {
    /// The stream's control register.
    pub const CTL: u32 = 0x00;
    /// The stream's status, in the top byte of the control register.
    pub const STS: u32 = 0x03;
    /// The link position: where the stream is in its buffer.
    pub const LPIB: u32 = 0x04;
    /// The cycle length: how many bytes the buffer holds.
    pub const CBL: u32 = 0x08;
    /// The link valid: how many descriptors the list holds, less one.
    pub const LVI: u32 = 0x0C;
    /// The size of the stream's buffer.
    pub const FIFOSIZE: u32 = 0x10;
    /// The format: the rate, the bits and the channels.
    pub const FORMAT: u32 = 0x12;
    /// The buffer descriptor list's base address, in the lower half.
    pub const BDLPL: u32 = 0x18;
    /// The buffer descriptor list's base address, in the upper half.
    pub const BDLPU: u32 = 0x1C;
}

/// `GCTL`'s controller reset bit: the controller is reset while it is set.
pub const GCTL_RESET: u32 = 1 << 0;
/// `GCTL`'s flush control bit.
pub const GCTL_FCNTRL: u32 = 1 << 1;
/// `GCTL`'s unsolicited response bit.
pub const GCTL_UNSOL: u32 = 1 << 8;
/// `CORBCTL`'s run bit: the controller takes verbs from the CORB.
pub const CORBCTL_RUN: u8 = 1 << 1;
/// `CORBCTL`'s memory error bit: the controller interrupts on a bad verb.
pub const CORBCTL_CMEIE: u8 = 1 << 0;
/// `RIRBCTL`'s interrupt bit: the controller interrupts on an answer.
pub const RIRBCTL_IRQ_EN: u8 = 1 << 0;
/// `RIRBCTL`'s run bit: the controller writes answers into the RIRB.
pub const RIRBCTL_DMA_EN: u8 = 1 << 1;
/// `RIRBCTL`'s overrun bit: the controller interrupts on a lost answer.
pub const RIRBCTL_OVERRUN_EN: u8 = 1 << 2;
/// `INTCTL`'s global interrupt enable.
pub const INTCTL_GLOBAL_EN: u32 = 1 << 31;
/// `INTCTL`'s controller interrupt enable.
pub const INTCTL_CONTROLLER_EN: u32 = 1 << 30;
/// `INTCTL`'s stream interrupt enables, one bit per stream.
pub const INTCTL_STREAM_EN: u32 = 0xFF;

/// The stream control register's stream reset bit.
pub const ST_CTL_RESET: u32 = 1 << 0;
/// The stream control register's run bit: the stream plays.
pub const ST_CTL_RUN: u32 = 1 << 1;
/// The stream control register's bidirectional bit.
pub const ST_CTL_BIDIRECTIONAL: u32 = 1 << 19;
/// The stream control register's tag bits: which tag the stream's interrupt
/// carries.
pub const ST_CTL_TAG_SHIFT: u32 = 20;
/// The stream control register's tag bits, as a mask.
pub const ST_CTL_TAG_MASK: u32 = 0x0F << 20;

/// The format of a stream: 48 kHz, sixteen bits, two channels.
///
/// The format register holds the channels in bits 3:0, the bits per sample
/// in bits 6:4, the multiplier in bits 13:11, the base rate in bit 14 and
/// the type in bit 15. Sixteen bits is `0b0001` in bits 6:4, the base rate
/// of 48 kHz is a zero in bit 14 and PCM a zero in bit 15.
pub const FORMAT_48K_S16_2CH: u16 = 0x0012;

/// The register block's offset of the stream `index`'s own block.
///
/// The first four streams take sound in and the next four play it out,
/// each in a block of its own.
#[must_use]
pub fn stream_base(index: u8) -> u32 {
    if index < 4 {
        register::STREAMS + u32::from(index) * register::STREAM_STRIDE
    } else {
        register::OUTPUT_STREAMS + u32::from(index - 4) * register::STREAM_STRIDE
    }
}

/// Why the controller would not do what was asked of it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    /// The controller did not come out of its reset.
    Reset,
    /// The CORB or the RIRB was not the size the driver expects.
    Geometry,
    /// The codec did not answer a verb.
    NoAnswer,
    /// The codec answered a verb with a failure.
    Refused,
}

/// The controller's register block, as the driver reads and writes it.
pub trait Mmio {
    /// The byte at `offset`.
    fn read8(&self, offset: u32) -> u8;
    /// The little-endian `u16` at `offset`.
    fn read16(&self, offset: u32) -> u16;
    /// The little-endian `u32` at `offset`.
    fn read32(&self, offset: u32) -> u32;
    /// Write the byte at `offset`.
    fn write8(&mut self, offset: u32, value: u8);
    /// Write the little-endian `u16` at `offset`.
    fn write16(&mut self, offset: u32, value: u16);
    /// Write the little-endian `u32` at `offset`.
    fn write32(&mut self, offset: u32, value: u32);
}

/// The controller: its register block and the verbs it runs.
pub struct Controller<M> {
    mmio: M,
}

impl<M> core::fmt::Debug for Controller<M> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Controller")
    }
}

impl<M: Mmio> Controller<M> {
    /// The controller over the register block `mmio` is.
    pub fn new(mmio: M) -> Self {
        Self { mmio }
    }

    /// The register block.
    pub fn mmio(&self) -> &M {
        &self.mmio
    }

    /// The register block, to write to.
    pub fn mmio_mut(&mut self) -> &mut M {
        &mut self.mmio
    }

    /// How many input streams the controller has.
    pub fn input_streams(&self) -> u8 {
        ((self.mmio.read16(register::GCAP) >> 8) & 0x0F) as u8
    }

    /// How many output streams the controller has.
    pub fn output_streams(&self) -> u8 {
        ((self.mmio.read16(register::GCAP) >> 12) & 0x0F) as u8
    }

    /// Whether the codec `cad` is out of its reset.
    ///
    /// The status register holds one bit per codec: it is set when the codec
    /// has finished resetting and can be talked to.
    pub fn codec_ready(&self, cad: u8) -> bool {
        self.mmio.read16(register::STATESTS) & (1 << cad) != 0
    }

    /// Take the controller through its reset and out of it.
    ///
    /// The reset bit is set, the codec's own state is waited for, and the
    /// bit is cleared again. The wait is `spins` reads of the status
    /// register, since the driver program's only clock is the reads
    /// themselves.
    ///
    /// # Errors
    ///
    /// [`Error::Reset`] if the codec was still in its reset when the reads
    /// ran out.
    pub fn reset(&mut self, spins: u32) -> Result<(), Error> {
        let gctl = self.mmio.read32(register::GCTL) | GCTL_RESET;
        self.mmio.write32(register::GCTL, gctl);
        if !self.ready(spins) {
            return Err(Error::Reset);
        }
        let gctl = self.mmio.read32(register::GCTL) & !GCTL_RESET;
        self.mmio.write32(register::GCTL, gctl);
        if !self.ready(spins) {
            return Err(Error::Reset);
        }
        Ok(())
    }

    /// Whether the codec came out of its reset within `spins` reads.
    fn ready(&self, spins: u32) -> bool {
        for _ in 0..spins {
            if self.codec_ready(0) {
                return true;
            }
        }
        false
    }

    /// The CORB's size, in entries.
    ///
    /// The size register's top bit says the register does not count the
    /// entries; a clear bit means the ring has the full 256.
    pub fn corb_entries(&self) -> u16 {
        let size = self.mmio.read8(register::CORBSIZE);
        if size & 0x80 == 0 {
            256
        } else {
            u16::from(size & 0x7F)
        }
    }

    /// The RIRB's size, in entries.
    pub fn rirb_entries(&self) -> u16 {
        let size = self.mmio.read8(register::RIRBSIZE);
        if size & 0x80 == 0 {
            256
        } else {
            u16::from(size & 0x7F)
        }
    }

    /// The CORB's base address, as the controller's DMA sees it.
    pub fn corb_base(&self) -> u64 {
        let low = u64::from(self.mmio.read32(register::CORBLBASE));
        let high = u64::from(self.mmio.read32(register::CORBUBASE));
        (high << 32) | low
    }

    /// The RIRB's base address, as the controller's DMA sees it.
    pub fn rirb_base(&self) -> u64 {
        let low = u64::from(self.mmio.read32(register::RIRBLBASE));
        let high = u64::from(self.mmio.read32(register::RIRBUBASE));
        (high << 32) | low
    }

    /// Let the controller take verbs from the CORB.
    pub fn open_corb(&mut self) {
        let ctl = self.mmio.read8(register::CORBCTL) | CORBCTL_RUN | CORBCTL_CMEIE;
        self.mmio.write8(register::CORBCTL, ctl);
    }

    /// Let the controller write answers into the RIRB.
    pub fn open_rirb(&mut self) {
        let ctl = self.mmio.read8(register::RIRBCTL)
            | RIRBCTL_DMA_EN
            | RIRBCTL_IRQ_EN
            | RIRBCTL_OVERRUN_EN;
        self.mmio.write8(register::RIRBCTL, ctl);
    }

    /// The interrupt status register, as the controller wrote it.
    pub fn interrupt_status(&self) -> u32 {
        self.mmio.read32(register::INTSTS)
    }

    /// Clear the interrupt status register's bits the driver has handled.
    pub fn clear_interrupts(&mut self, bits: u32) {
        let status = self.interrupt_status() & bits;
        self.mmio.write32(register::INTSTS, status);
    }

    /// The register block's offset of the stream `index`'s own block.
    ///
    /// The first four streams take sound in and the next four play it out,
    /// each in a block of its own.
    pub fn stream_base(index: u8) -> u32 {
        stream_base(index)
    }

    /// The stream's control register, in the stream's own block.
    pub fn stream_ctl(index: u8) -> u32 {
        Self::stream_base(index) + stream::CTL
    }

    /// The stream's buffer descriptor list's base address.
    pub fn stream_bdl_base(&self, index: u8) -> u64 {
        let base = Self::stream_base(index);
        let low = u64::from(self.mmio.read32(base + stream::BDLPL));
        let high = u64::from(self.mmio.read32(base + stream::BDLPU));
        (high << 32) | low
    }

    /// Set the stream's buffer descriptor list's base address.
    pub fn set_stream_bdl_base(&mut self, index: u8, address: u64) {
        let base = Self::stream_base(index);
        self.mmio
            .write32(base + stream::BDLPL, (address & 0xFFFF_FFFF) as u32);
        self.mmio.write32(base + stream::BDLPU, (address >> 32) as u32);
    }
}
