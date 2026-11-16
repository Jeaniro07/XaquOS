use crate::arch;
use crate::sync::SpinLock;
use xaqu_bootinfo::BootInfo;
use xaqu_bootinfo::FIRMWARE_SEED;
use xaqu_crng::Crng;
use xaqu_crng::KEY_BYTES;
pub fn fill(bytes: &mut [u8]) {
    let _ = xaqu_native::lift_calls::random_fill(xaqu_rt::Kernel, bytes);
}
