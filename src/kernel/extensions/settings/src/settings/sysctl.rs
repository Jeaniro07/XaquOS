//! The writable `/proc/sys/kernel` values, `hostname` and `domainname`
//! (moved whole from `fs/procfs/render.rs`; `docs/KERNEL-CRATES.md` §6.2).
//! The names stay their namespace's, in the load.

use xaqu_kernel_load::fs::procfs::Kernel;
use xaqu_kernel_load::syscall::nsproxy;
use xaqu_kernel_load::syscall::system::NAME_MAX;
use xaqu_procfs::sysctl;
use xaqu_vfs::Result;

/// A write to `kernel/hostname`: `proc_dostring`'s, so `echo name >` stores
/// `name`, and a name past `__NEW_UTS_LEN` is cut there rather than refused,
/// which is where the sysctl and `sethostname` differ on Linux too. The
/// write is taken as from the start of the value, whatever the offset.
pub fn set_hostname(_: &Kernel, data: &[u8]) -> Result<usize> {
    nsproxy::acting()
        .uts
        .set_hostname(sysctl::stored(data, NAME_MAX))?;
    Ok(data.len())
}

/// A write to `kernel/domainname`, as [`set_hostname`].
pub fn set_domainname(_: &Kernel, data: &[u8]) -> Result<usize> {
    nsproxy::acting()
        .uts
        .set_domainname(sysctl::stored(data, NAME_MAX))?;
    Ok(data.len())
}
