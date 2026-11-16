//! linuxd's shim of the kernel's core (docs/microkernel/M5-LIFT.md): the core's
//! `pub` interface as the load names it, implemented in user space, so that
//! the load's one source builds into ring 0 for `desktop` and into linuxd for
//! `fast` and `max`.

#![no_std]
#![allow(
    unused_qualifications,
    unused_unsafe,
    unused_extern_crates,
    unused_crate_dependencies,
    dead_code,
    unused_imports,
    unused_variables,
    missing_docs,
    missing_debug_implementations,
    unreachable_pub,
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    reason = "the shim's stubs, until each milestone reaches them"
)]

extern crate alloc;

#[macro_export]
macro_rules! build_flag_word {
    () => {
        [
            cfg!(xaqu_profile = "present"),
            cfg!(xaqu_profile = "desktop"),
            cfg!(xaqu_profile = "fast"),
            cfg!(xaqu_profile = "max"),
            cfg!(xaqu_profile = "none"),
            cfg!(xaqu_level = "none"),
            cfg!(xaqu_level = "present"),
            cfg!(xaqu_level = "sc2"),
            cfg!(xaqu_level = "sc4"),
            cfg!(xaqu_personality = "user"),
            cfg!(xaqu_personality = "kernel"),
            cfg!(xaqu_personality = "none"),
            cfg!(xaqu_fs = "server"),
            cfg!(xaqu_fs = "kernel"),
            cfg!(xaqu_fs = "none"),
            cfg!(xaqu_fs = "lift"),
            cfg!(xaqu_mitigations = "on"),
            cfg!(xaqu_mitigations = "off"),
            cfg!(xaqu_partition_speculation = "exit"),
            cfg!(xaqu_partition_speculation = "partition"),
            cfg!(xaqu_partition_speculation = "not_applicable"),
            cfg!(xaqu_ipc_fastpath = "off"),
            cfg!(xaqu_ipc_fastpath = "on"),
            cfg!(xaqu_restricted_fastpath = "off"),
            cfg!(xaqu_restricted_fastpath = "on"),
            cfg!(xaqu_restricted_fastpath = "not_applicable"),
            cfg!(xaqu_fault_ahead = "off"),
            cfg!(xaqu_fault_ahead = "on"),
            cfg!(xaqu_fault_ahead = "not_applicable"),
            cfg!(xaqu_restricted_window = "off"),
            cfg!(xaqu_restricted_window = "on"),
            cfg!(xaqu_restricted_window = "not_applicable"),
            cfg!(xaqu_kernel_memory = "retyped"),
            cfg!(xaqu_kernel_memory = "heap"),
            cfg!(xaqu_schedule = "windows"),
            cfg!(xaqu_schedule = "edf"),
            cfg!(xaqu_schedule = "fair"),
            cfg!(xaqu_checks = "on"),
            cfg!(xaqu_checks = "off"),
            cfg!(xaqu_program_barrier = "switch"),
            cfg!(xaqu_program_barrier = "session"),
            cfg!(xaqu_ssbd = "always"),
            cfg!(xaqu_ssbd = "on_request"),
            cfg!(xaqu_smt = "off"),
            cfg!(xaqu_smt = "stibp"),
            cfg!(xaqu_runtime_trades = "none"),
            cfg!(xaqu_runtime_trades = "game_mode"),
            cfg!(xaqu_linux_abi = "on"),
            cfg!(xaqu_linux_abi = "off"),
            cfg!(xaqu_runtime_settings = "fixed"),
            cfg!(xaqu_runtime_settings = "live"),
        ]
    };
}

/// [`build_flag_word!`]'s values as bits, the first lowest.
#[must_use]
pub const fn flag_word(chosen: &[bool]) -> u64 {
    let mut word = 0_u64;
    let mut at = 0;
    let mut rest = chosen;
    while let [set, tail @ ..] = rest {
        if *set && at < 64 {
            word |= 1 << at;
        }
        at += 1;
        rest = tail;
    }
    word
}

/// The shim's own modules, found by position under `src/kernel/src/lift/core`.
pub mod lift {
    pub mod core {
        pub mod arch;
        pub mod audit;
        pub mod console;
        pub mod device;
        pub mod discovery;

        pub mod iommu;
        pub mod irq;
        pub mod mm;
        pub mod mmio;
        pub mod panic;
        pub mod sched;

        pub mod bridge;
        pub mod calls_check;
        pub mod fill;
        pub mod heap;
        pub mod restricted;
        pub mod smp;
        pub mod sync;
        pub mod sys;
        pub mod timer;
        pub mod trap;
        pub mod vmap;
        pub mod watchdog;
    }
}
pub use lift::core::{
    audit, bridge, device, discovery, fill, heap, iommu, irq, mm, mmio, restricted, sched, smp,
    sync, sys, timer, trap, vmap, watchdog,
};
pub mod claim;
pub mod fallible;
pub mod hooks;
pub mod signal_frame;
/// The architecture modules: the shim's, and the files that are the same source.
pub mod arch {
    pub use crate::lift::core::arch::*;
    #[cfg(target_arch = "x86_64")]
    pub use x86_64::vdso::{vdso_can_read_counter, vdso_spec};
    /// x86-64.
    #[cfg(target_arch = "x86_64")]
    pub mod x86_64 {
        pub use crate::lift::core::arch::x86_64::*;
        pub mod signal;
        /// The vDSO's code is a program's, never run here: the image linuxd maps
        /// (lift_reuse's one file holding `global_asm!`).
        pub mod vdso;
    }
    /// AArch64.
    #[cfg(target_arch = "aarch64")]
    pub mod aarch64 {
        pub use crate::lift::core::arch::aarch64::*;
        pub mod signal;
    }
}
/// The console: the shim's, and the log ring, which is the same source.
pub mod console {
    pub use crate::lift::core::console::*;
    pub mod log;
}
/// The panic catalog is the same source.
pub mod panic {
    pub use crate::lift::core::panic::*;
    pub mod catalog;
}
/// The user-memory copies are the same source.
pub mod syscall {
    pub mod uaccess;
}
pub mod object;
pub mod user;
