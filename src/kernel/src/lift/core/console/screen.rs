use core::hint::spin_loop;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use xaqu_bootinfo::BootView;
use xaqu_fbtext::GLYPH_HEIGHT;
use xaqu_fbtext::GLYPH_WIDTH;
use xaqu_fbtext::Rgb;
use xaqu_fbtext::Surface;
use xaqu_sync::SpinLock;
pub fn stop() {}
