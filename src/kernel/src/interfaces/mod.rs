//! The kernel's interfaces to ring-3 drivers: one core per kind of device.
//!
//! Nothing here drives hardware. A driver is a process under
//! `src/user/system/native/drivers/`; what lives here is the kernel's end of the
//! control channel or ring that driver speaks, checked word for word against
//! its protocol crate in `src/lib/proto/`, and what the kernel publishes from
//! it to programs: a disk, a network interface, a card under `/dev/dri`, an
//! event node under `/dev/input`, a sound card, the kernel log's reader.
//!
//! | Core | Protocol | Design |
//! |---|---|---|
//! | [`block_ring`] | `xaqu-blkring` | `docs/BLOCK-RING.md` |
//! | `net_ring`, the part `xaqu-kernel-net-ring` (CRATE-SPLIT.md S4) | `xaqu-netring` | `docs/NET-RING.md` |
//! | [`display`] | `xaqu-displayctl` | `docs/DISPLAY.md` |
//! | [`render`] | `xaqu-renderctl` | `docs/GPU.md` |
//! | [`input`] | `xaqu-inputctl` | `docs/INPUT.md` |
//! | [`audio`] | `xaqu-sndctl` | `docs/AUDIO.md` |
//! | `logctl`, the part `xaqu-kernel-logctl` (CRATE-SPLIT.md S3) | `xaqu-logctl` | the kernel log |
//!
//! The certification item puts every core in the load ring
//! (`tools/common/data/certification-item.json`): a defect in one is bounded
//! by the item's own enforcement, as a driver's fault is.

pub mod audio;
pub mod block_ring;
pub mod chardev;
pub mod display;
pub mod input;
pub mod render;
