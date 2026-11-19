//! The iwlwifi firmware file: its fixed header and its TLV directory.
//!
//! An iwlwifi firmware file (a `.ucode` file) is a fixed header followed by a
//! directory of type-length-value records. This module reads that format and
//! nothing else: it does not decompress a `.zst` firmware, does not talk to
//! the device, and does not decide which file to load.
//!
//! The format is the Linux kernel's, read from
//! `drivers/net/wireless/intel/iwlwifi/fw/file.h` (Linux v6.6):
//!
//! * `struct iwl_tlv_ucode_header` — the fixed header: a `zero` word that
//!   must be zero (which is how the TLV layout is told apart from the older
//!   v1/v2 layout of `struct iwl_ucode_header`), the `magic` word
//!   (`IWL_TLV_UCODE_MAGIC`), a 64-byte human-readable version string, the
//!   `ver` and `build` words and an eight-byte field the kernel ignores.
//! * `struct iwl_ucode_tlv` — one directory entry: a little-endian `type`,
//!   a little-endian `length` that does not count the two header words, and
//!   the data. Each record is padded so that the next one starts on a
//!   four-byte boundary.
//! * `enum iwl_ucode_tlv_type` — the record types.
//! * `enum iwl_ucode_tlv_flag` — the bits of the `FLAGS` record.
//!
//! The version word packs four values, which the kernel reads with
//! `IWL_UCODE_MAJOR`, `IWL_UCODE_MINOR`, `IWL_UCODE_API` and
//! `IWL_UCODE_SERIAL`; [`FirmwareHeader::major`] and its siblings do the
//! same.

use core::fmt;

/// The magic word at the head of a TLV-style firmware file.
///
/// Upstream: `IWL_TLV_UCODE_MAGIC`, `fw/file.h`.
pub const UCODE_MAGIC: u32 = 0x0a4c_5749;

/// The length of the human-readable version string in the header.
///
/// Upstream: `FW_VER_HUMAN_READABLE_SZ`, `fw/file.h`.
pub const HUMAN_READABLE_LEN: usize = 64;

/// The length of the fixed header of a TLV-style firmware file.
///
/// Four bytes of `zero`, four of `magic`, 64 of `human_readable`, four of
/// `ver`, four of `build` and eight of `ignore`.
pub const HEADER_LEN: usize = 88;

/// The length of one directory entry: a type word and a length word.
///
/// Upstream: `struct iwl_ucode_tlv`, `fw/file.h`.
pub const TLV_HEADER_LEN: usize = 8;

/// The base of the debug record types (`IWL_UCODE_TLV_DEBUG_BASE`).
const DEBUG_BASE: u32 = 0x100_0005;

/// The base of the constant record types (`IWL_UCODE_TLV_CONST_BASE`).
const CONST_BASE: u32 = 0x100;

/// Why a firmware file could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The buffer is shorter than the fixed header.
    TruncatedHeader {
        /// The number of bytes the header needs.
        needed: usize,
        /// The number of bytes the buffer holds.
        found: usize,
    },
    /// The first word is not zero, so this is the older v1/v2 layout rather
    /// than the TLV layout.
    LegacyLayout,
    /// The magic word is not [`UCODE_MAGIC`].
    BadMagic {
        /// The word that was found.
        found: u32,
    },
    /// A directory entry's data runs past the end of the buffer.
    TruncatedTlv {
        /// The offset of the entry in the file.
        offset: usize,
        /// The number of bytes the entry needs.
        needed: usize,
        /// The number of bytes the buffer holds from that offset.
        found: usize,
    },
    /// A directory entry's length word is larger than the buffer.
    BadTlvLength {
        /// The offset of the entry in the file.
        offset: usize,
        /// The length word that was read.
        length: u32,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TruncatedHeader { needed, found } => {
                write!(f, "firmware header needs {needed} bytes, got {found}")
            }
            Error::LegacyLayout => write!(f, "firmware is not in the TLV layout"),
            Error::BadMagic { found } => {
                write!(f, "firmware magic is {found:#010x}, expected {UCODE_MAGIC:#010x}")
            }
            Error::TruncatedTlv { offset, needed, found } => write!(
                f,
                "firmware record at {offset} needs {needed} bytes, got {found}"
            ),
            Error::BadTlvLength { offset, length } => {
                write!(f, "firmware record at {offset} claims {length} bytes")
            }
        }
    }
}

impl core::error::Error for Error {}

/// The type of one directory entry.
///
/// The named variants are the ones `enum iwl_ucode_tlv_type` names in the
/// kernel; anything else is [`TlvType::Other`]. Note that upstream gives
/// the value 7 two names — `IWL_UCODE_TLV_PAN`, deprecated and used by the
/// DVM stack only, and `IWL_UCODE_TLV_MEM_DESC`, which replaces it — and
/// this crate keeps the second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlvType {
    /// `IWL_UCODE_TLV_INVALID` (0), unused.
    Invalid,
    /// `IWL_UCODE_TLV_INST` (1), the runtime instructions.
    Inst,
    /// `IWL_UCODE_TLV_DATA` (2), the runtime data.
    Data,
    /// `IWL_UCODE_TLV_INIT` (3), the init instructions.
    Init,
    /// `IWL_UCODE_TLV_INIT_DATA` (4), the init data.
    InitData,
    /// `IWL_UCODE_TLV_BOOT` (5), the bootstrap code.
    Boot,
    /// `IWL_UCODE_TLV_PROBE_MAX_LEN` (6), a `u32`.
    ProbeMaxLen,
    /// `IWL_UCODE_TLV_MEM_DESC` (7), the memory descriptor.
    MemDesc,
    /// `IWL_UCODE_TLV_RUNT_EVTLOG_PTR` (8).
    RuntimeEvtlogPtr,
    /// `IWL_UCODE_TLV_RUNT_EVTLOG_SIZE` (9).
    RuntimeEvtlogSize,
    /// `IWL_UCODE_TLV_RUNT_ERRLOG_PTR` (10).
    RuntimeErrlogPtr,
    /// `IWL_UCODE_TLV_INIT_EVTLOG_PTR` (11).
    InitEvtlogPtr,
    /// `IWL_UCODE_TLV_INIT_EVTLOG_SIZE` (12).
    InitEvtlogSize,
    /// `IWL_UCODE_TLV_INIT_ERRLOG_PTR` (13).
    InitErrlogPtr,
    /// `IWL_UCODE_TLV_ENHANCE_SENS_TBL` (14).
    EnhanceSensTable,
    /// `IWL_UCODE_TLV_PHY_CALIBRATION_SIZE` (15), a `u32`.
    PhyCalibrationSize,
    /// `IWL_UCODE_TLV_WOWLAN_INST` (16).
    WowlanInst,
    /// `IWL_UCODE_TLV_WOWLAN_DATA` (17).
    WowlanData,
    /// `IWL_UCODE_TLV_FLAGS` (18), a bitmap of [`UcodeFlags`].
    Flags,
    /// `IWL_UCODE_TLV_SEC_RT` (19), the runtime security section.
    SecurityRuntime,
    /// `IWL_UCODE_TLV_SEC_INIT` (20), the init security section.
    SecurityInit,
    /// `IWL_UCODE_TLV_SEC_WOWLAN` (21), the WoWLAN security section.
    SecurityWowlan,
    /// `IWL_UCODE_TLV_DEF_CALIB` (22), the default calibration table.
    DefaultCalib,
    /// `IWL_UCODE_TLV_PHY_SKU` (23).
    PhySku,
    /// `IWL_UCODE_TLV_SECURE_SEC_RT` (24).
    SecureSecRuntime,
    /// `IWL_UCODE_TLV_SECURE_SEC_INIT` (25).
    SecureSecInit,
    /// `IWL_UCODE_TLV_SECURE_SEC_WOWLAN` (26).
    SecureSecWowlan,
    /// `IWL_UCODE_TLV_NUM_OF_CPU` (27).
    NumOfCpu,
    /// `IWL_UCODE_TLV_CSCHEME` (28).
    Cscheme,
    /// `IWL_UCODE_TLV_API_CHANGES_SET` (29).
    ApiChangesSet,
    /// `IWL_UCODE_TLV_ENABLED_CAPABILITIES` (30).
    EnabledCapabilities,
    /// `IWL_UCODE_TLV_N_SCAN_CHANNELS` (31), a `u32`.
    NumScanChannels,
    /// `IWL_UCODE_TLV_PAGING` (32).
    Paging,
    /// `IWL_UCODE_TLV_SEC_RT_USNIFFER` (34).
    SecurityRuntimeUsniffer,
    /// `IWL_UCODE_TLV_FW_VERSION` (36).
    FirmwareVersion,
    /// `IWL_UCODE_TLV_FW_DBG_DEST` (38).
    DebugDestination,
    /// `IWL_UCODE_TLV_FW_DBG_CONF` (39).
    DebugConfiguration,
    /// `IWL_UCODE_TLV_FW_DBG_TRIGGER` (40).
    DebugTrigger,
    /// `IWL_UCODE_TLV_CMD_VERSIONS` (48).
    CommandVersions,
    /// `IWL_UCODE_TLV_FW_GSCAN_CAPA` (50).
    GscanCapabilities,
    /// `IWL_UCODE_TLV_FW_MEM_SEG` (51).
    FirmwareMemorySegment,
    /// `IWL_UCODE_TLV_IML` (52), the image loader.
    ImageLoader,
    /// `IWL_UCODE_TLV_UMAC_DEBUG_ADDRS` (54).
    UmacDebugAddresses,
    /// `IWL_UCODE_TLV_LMAC_DEBUG_ADDRS` (55).
    LmacDebugAddresses,
    /// `IWL_UCODE_TLV_FW_RECOVERY_INFO` (57).
    RecoveryInfo,
    /// `IWL_UCODE_TLV_HW_TYPE` (58).
    HardwareType,
    /// `IWL_UCODE_TLV_FW_FSEQ_VERSION` (60).
    FseqVersion,
    /// `IWL_UCODE_TLV_PHY_INTEGRATION_VERSION` (61).
    PhyIntegrationVersion,
    /// `IWL_UCODE_TLV_PNVM_VERSION` (62).
    PnvmVersion,
    /// `IWL_UCODE_TLV_PNVM_SKU` (64).
    PnvmSku,
    /// `IWL_UCODE_TLV_SEC_TABLE_ADDR` (66).
    SecurityTableAddress,
    /// `IWL_UCODE_TLV_D3_KEK_KCK_ADDR` (67).
    D3KekKckAddress,
    /// `IWL_UCODE_TLV_CURRENT_PC` (68).
    CurrentPc,
    /// `IWL_UCODE_TLV_FW_NUM_STATIONS` (0x100).
    NumberOfStations,
    /// `IWL_UCODE_TLV_FW_NUM_BEACONS` (0x102).
    NumberOfBeacons,
    /// `IWL_UCODE_TLV_TYPE_DEBUG_INFO` (0x1000005).
    DebugInfo,
    /// `IWL_UCODE_TLV_TYPE_BUFFER_ALLOCATION` (0x1000006).
    BufferAllocation,
    /// `IWL_UCODE_TLV_TYPE_HCMD` (0x1000007).
    HostCommand,
    /// `IWL_UCODE_TLV_TYPE_REGIONS` (0x1000008).
    Regions,
    /// `IWL_UCODE_TLV_TYPE_TRIGGERS` (0x1000009).
    Triggers,
    /// `IWL_UCODE_TLV_TYPE_CONF_SET` (0x100000a).
    ConfigurationSet,
    /// `IWL_UCODE_TLV_FW_DBG_DUMP_LST` (0x1000), for the driver's own use.
    DebugDumpList,
    /// A type the kernel's enum does not name.
    Other(u32),
}

impl TlvType {
    /// The type with the given raw value.
    pub fn from_raw(raw: u32) -> TlvType {
        match raw {
            0 => TlvType::Invalid,
            1 => TlvType::Inst,
            2 => TlvType::Data,
            3 => TlvType::Init,
            4 => TlvType::InitData,
            5 => TlvType::Boot,
            6 => TlvType::ProbeMaxLen,
            7 => TlvType::MemDesc,
            8 => TlvType::RuntimeEvtlogPtr,
            9 => TlvType::RuntimeEvtlogSize,
            10 => TlvType::RuntimeErrlogPtr,
            11 => TlvType::InitEvtlogPtr,
            12 => TlvType::InitEvtlogSize,
            13 => TlvType::InitErrlogPtr,
            14 => TlvType::EnhanceSensTable,
            15 => TlvType::PhyCalibrationSize,
            16 => TlvType::WowlanInst,
            17 => TlvType::WowlanData,
            18 => TlvType::Flags,
            19 => TlvType::SecurityRuntime,
            20 => TlvType::SecurityInit,
            21 => TlvType::SecurityWowlan,
            22 => TlvType::DefaultCalib,
            23 => TlvType::PhySku,
            24 => TlvType::SecureSecRuntime,
            25 => TlvType::SecureSecInit,
            26 => TlvType::SecureSecWowlan,
            27 => TlvType::NumOfCpu,
            28 => TlvType::Cscheme,
            29 => TlvType::ApiChangesSet,
            30 => TlvType::EnabledCapabilities,
            31 => TlvType::NumScanChannels,
            32 => TlvType::Paging,
            34 => TlvType::SecurityRuntimeUsniffer,
            36 => TlvType::FirmwareVersion,
            38 => TlvType::DebugDestination,
            39 => TlvType::DebugConfiguration,
            40 => TlvType::DebugTrigger,
            48 => TlvType::CommandVersions,
            50 => TlvType::GscanCapabilities,
            51 => TlvType::FirmwareMemorySegment,
            52 => TlvType::ImageLoader,
            54 => TlvType::UmacDebugAddresses,
            55 => TlvType::LmacDebugAddresses,
            57 => TlvType::RecoveryInfo,
            58 => TlvType::HardwareType,
            60 => TlvType::FseqVersion,
            61 => TlvType::PhyIntegrationVersion,
            62 => TlvType::PnvmVersion,
            64 => TlvType::PnvmSku,
            66 => TlvType::SecurityTableAddress,
            67 => TlvType::D3KekKckAddress,
            68 => TlvType::CurrentPc,
            0x1000 => TlvType::DebugDumpList,
            0x1000005 => TlvType::DebugInfo,
            0x1000006 => TlvType::BufferAllocation,
            0x1000007 => TlvType::HostCommand,
            0x1000008 => TlvType::Regions,
            0x1000009 => TlvType::Triggers,
            0x100000a => TlvType::ConfigurationSet,
            other => TlvType::Other(other),
        }
    }

    /// The raw value of this type, as it appears in the file.
    pub fn raw(self) -> u32 {
        match self {
            TlvType::Invalid => 0,
            TlvType::Inst => 1,
            TlvType::Data => 2,
            TlvType::Init => 3,
            TlvType::InitData => 4,
            TlvType::Boot => 5,
            TlvType::ProbeMaxLen => 6,
            TlvType::MemDesc => 7,
            TlvType::RuntimeEvtlogPtr => 8,
            TlvType::RuntimeEvtlogSize => 9,
            TlvType::RuntimeErrlogPtr => 10,
            TlvType::InitEvtlogPtr => 11,
            TlvType::InitEvtlogSize => 12,
            TlvType::InitErrlogPtr => 13,
            TlvType::EnhanceSensTable => 14,
            TlvType::PhyCalibrationSize => 15,
            TlvType::WowlanInst => 16,
            TlvType::WowlanData => 17,
            TlvType::Flags => 18,
            TlvType::SecurityRuntime => 19,
            TlvType::SecurityInit => 20,
            TlvType::SecurityWowlan => 21,
            TlvType::DefaultCalib => 22,
            TlvType::PhySku => 23,
            TlvType::SecureSecRuntime => 24,
            TlvType::SecureSecInit => 25,
            TlvType::SecureSecWowlan => 26,
            TlvType::NumOfCpu => 27,
            TlvType::Cscheme => 28,
            TlvType::ApiChangesSet => 29,
            TlvType::EnabledCapabilities => 30,
            TlvType::NumScanChannels => 31,
            TlvType::Paging => 32,
            TlvType::SecurityRuntimeUsniffer => 34,
            TlvType::FirmwareVersion => 36,
            TlvType::DebugDestination => 38,
            TlvType::DebugConfiguration => 39,
            TlvType::DebugTrigger => 40,
            TlvType::CommandVersions => 48,
            TlvType::GscanCapabilities => 50,
            TlvType::FirmwareMemorySegment => 51,
            TlvType::ImageLoader => 52,
            TlvType::UmacDebugAddresses => 54,
            TlvType::LmacDebugAddresses => 55,
            TlvType::RecoveryInfo => 57,
            TlvType::HardwareType => 58,
            TlvType::FseqVersion => 60,
            TlvType::PhyIntegrationVersion => 61,
            TlvType::PnvmVersion => 62,
            TlvType::PnvmSku => 64,
            TlvType::SecurityTableAddress => 66,
            TlvType::D3KekKckAddress => 67,
            TlvType::CurrentPc => 68,
            TlvType::NumberOfStations => CONST_BASE,
            TlvType::NumberOfBeacons => CONST_BASE + 2,
            TlvType::DebugInfo => DEBUG_BASE,
            TlvType::BufferAllocation => DEBUG_BASE + 1,
            TlvType::HostCommand => DEBUG_BASE + 2,
            TlvType::Regions => DEBUG_BASE + 3,
            TlvType::Triggers => DEBUG_BASE + 4,
            TlvType::ConfigurationSet => DEBUG_BASE + 5,
            TlvType::DebugDumpList => 0x1000,
            TlvType::Other(other) => other,
        }
    }
}

/// The bits of the `FLAGS` record.
///
/// Upstream: `enum iwl_ucode_tlv_flag`, `fw/file.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UcodeFlags(u32);

impl UcodeFlags {
    /// No bit set.
    pub const NONE: UcodeFlags = UcodeFlags(0);
    /// `IWL_UCODE_TLV_FLAGS_PAN`: the image is PAN capable.
    pub const PAN: UcodeFlags = UcodeFlags(1 << 0);
    /// `IWL_UCODE_TLV_FLAGS_NEWSCAN`: new scan behaviour on hidden SSIDs.
    pub const NEWSCAN: UcodeFlags = UcodeFlags(1 << 1);
    /// `IWL_UCODE_TLV_FLAGS_MFP`: the image supports management frame
    /// protection (802.11w).
    pub const MFP: UcodeFlags = UcodeFlags(1 << 2);
    /// `IWL_UCODE_TLV_FLAGS_SHORT_BL`: sixteen block-list entries, not 64.
    pub const SHORT_BL: UcodeFlags = UcodeFlags(1 << 7);
    /// `IWL_UCODE_TLV_FLAGS_D3_6_IPV6_ADDRS`: the D3 image supports six
    /// IPv6 addresses rather than two.
    pub const D3_6_IPV6_ADDRS: UcodeFlags = UcodeFlags(1 << 10);
    /// `IWL_UCODE_TLV_FLAGS_NO_BASIC_SSID`: no probe carries the SSID
    /// element.
    pub const NO_BASIC_SSID: UcodeFlags = UcodeFlags(1 << 12);
    /// `IWL_UCODE_TLV_FLAGS_NEW_NSOFFL_SMALL`: the small new NS offload.
    pub const NEW_NSOFFL_SMALL: UcodeFlags = UcodeFlags(1 << 15);
    /// `IWL_UCODE_TLV_FLAGS_NEW_NSOFFL_LARGE`: the large new NS offload.
    pub const NEW_NSOFFL_LARGE: UcodeFlags = UcodeFlags(1 << 16);
    /// `IWL_UCODE_TLV_FLAGS_UAPSD_SUPPORT`: the image supports uAPSD.
    pub const UAPSD_SUPPORT: UcodeFlags = UcodeFlags(1 << 24);
    /// `IWL_UCODE_TLV_FLAGS_EBS_SUPPORT`: the image supports EBS.
    pub const EBS_SUPPORT: UcodeFlags = UcodeFlags(1 << 25);
    /// `IWL_UCODE_TLV_FLAGS_P2P_PS_UAPSD`: a P2P client supports uAPSD.
    pub const P2P_PS_UAPSD: UcodeFlags = UcodeFlags(1 << 26);

    /// The raw bitmap.
    pub fn raw(self) -> u32 {
        self.0
    }

    /// Whether every bit of `other` is set in this bitmap.
    pub fn contains(self, other: UcodeFlags) -> bool {
        self.0 & other.0 == other.0
    }
}

/// The fixed header at the start of a TLV-style firmware file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareHeader {
    version: u32,
    build: u32,
    human_readable: [u8; HUMAN_READABLE_LEN],
}

impl FirmwareHeader {
    /// Reads the header out of the start of a firmware file.
    pub fn parse(bytes: &[u8]) -> Result<FirmwareHeader, Error> {
        if bytes.len() < HEADER_LEN {
            return Err(Error::TruncatedHeader {
                needed: HEADER_LEN,
                found: bytes.len(),
            });
        }
        let zero = read_u32(bytes, 0);
        if zero != 0 {
            return Err(Error::LegacyLayout);
        }
        let magic = read_u32(bytes, 4);
        if magic != UCODE_MAGIC {
            return Err(Error::BadMagic { found: magic });
        }
        let readable = bytes
            .get(8..8 + HUMAN_READABLE_LEN)
            .and_then(|field| <[u8; HUMAN_READABLE_LEN]>::try_from(field).ok())
            .ok_or(Error::TruncatedHeader {
                needed: HEADER_LEN,
                found: bytes.len(),
            })?;
        Ok(FirmwareHeader {
            version: read_u32(bytes, 72),
            build: read_u32(bytes, 76),
            human_readable: readable,
        })
    }

    /// The version word, packing major, minor, API and serial.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// The build number.
    pub fn build(&self) -> u32 {
        self.build
    }

    /// The major version (`IWL_UCODE_MAJOR`).
    pub fn major(&self) -> u8 {
        ((self.version & 0xff00_0000) >> 24) as u8
    }

    /// The minor version (`IWL_UCODE_MINOR`).
    pub fn minor(&self) -> u8 {
        ((self.version & 0x00ff_0000) >> 16) as u8
    }

    /// The firmware API version (`IWL_UCODE_API`).
    pub fn api(&self) -> u8 {
        ((self.version & 0x0000_ff00) >> 8) as u8
    }

    /// The serial number (`IWL_UCODE_SERIAL`).
    pub fn serial(&self) -> u8 {
        (self.version & 0x0000_00ff) as u8
    }

    /// The human-readable version string, up to its first zero byte.
    pub fn human_readable(&self) -> &str {
        let end = self
            .human_readable
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(HUMAN_READABLE_LEN);
        core::str::from_utf8(self.human_readable.get(..end).unwrap_or(&[])).unwrap_or("")
    }
}

/// One directory entry of a firmware file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tlv<'a> {
    kind: TlvType,
    data: &'a [u8],
}

impl<'a> Tlv<'a> {
    /// The type of the entry.
    pub fn kind(&self) -> TlvType {
        self.kind
    }

    /// The data of the entry, without its header.
    pub fn data(&self) -> &'a [u8] {
        self.data
    }
}

/// The directory of a firmware file, read entry by entry.
///
/// Each item is a [`Result`]: a file whose last entry claims more bytes than
/// it has is an error, not a silent end.
#[derive(Debug, Clone)]
pub struct TlvIter<'a> {
    rest: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for TlvIter<'a> {
    type Item = Result<Tlv<'a>, Error>;

    fn next(&mut self) -> Option<Result<Tlv<'a>, Error>> {
        if self.rest.len() < TLV_HEADER_LEN {
            return None;
        }
        let kind = TlvType::from_raw(read_u32(self.rest, 0));
        let length = read_u32(self.rest, 4) as usize;
        let available = self.rest.len() - TLV_HEADER_LEN;
        if length > available {
            return Some(Err(Error::BadTlvLength {
                offset: self.offset,
                length: length as u32,
            }));
        }
        let data = self
            .rest
            .get(TLV_HEADER_LEN..TLV_HEADER_LEN + length)
            .unwrap_or(&[]);
        let padded = TLV_HEADER_LEN + length.div_ceil(4) * 4;
        self.offset += padded;
        self.rest = if padded >= self.rest.len() {
            &[]
        } else {
            self.rest.get(padded..).unwrap_or(&[])
        };
        Some(Ok(Tlv { kind, data }))
    }
}

/// A firmware file: its header and its directory.
#[derive(Debug, Clone)]
pub struct FirmwareFile<'a> {
    header: FirmwareHeader,
    body: &'a [u8],
}

impl<'a> FirmwareFile<'a> {
    /// Reads a firmware file: the header, and the directory that follows it.
    pub fn parse(bytes: &'a [u8]) -> Result<FirmwareFile<'a>, Error> {
        let header = FirmwareHeader::parse(bytes)?;
        let body = bytes.get(HEADER_LEN..).unwrap_or(&[]);
        Ok(FirmwareFile { header, body })
    }

    /// The header of the file.
    pub fn header(&self) -> &FirmwareHeader {
        &self.header
    }

    /// The directory of the file, read entry by entry.
    pub fn tlvs(&self) -> TlvIter<'a> {
        TlvIter {
            rest: self.body,
            offset: HEADER_LEN,
        }
    }

    /// The data of the first entry of the given type.
    pub fn tlv(&self, kind: TlvType) -> Option<&'a [u8]> {
        self.tlvs()
            .find_map(|entry| match entry {
                Ok(entry) if entry.kind == kind => Some(entry.data),
                Ok(_) => None,
                Err(_) => None,
            })
    }

    /// The data of the first entry of the given type, read as a `u32`.
    pub fn tlv_u32(&self, kind: TlvType) -> Option<u32> {
        self.tlv(kind)
            .filter(|data| data.len() >= 4)
            .map(|data| read_u32(data, 0))
    }
}

/// Reads the little-endian `u32` at `at`, or zero if the bytes are not there.
fn read_u32(bytes: &[u8], at: usize) -> u32 {
    bytes
        .get(at..at.saturating_add(4))
        .and_then(|field| <[u8; 4]>::try_from(field).ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0)
}
