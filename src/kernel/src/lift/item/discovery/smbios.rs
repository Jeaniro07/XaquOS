use xaqu_bootinfo::BootView;
use xaqu_bootinfo::MemKind;
use xaqu_bootinfo::MemRegion;
use xaqu_bootinfo::SMBIOS_ENTRY_BYTES;
use xaqu_smbios::Error;
use xaqu_smbios::Smbios;
use xaqu_smbios::TABLE_MAX;
use xaqu_smbios::Value;
use xaqu_sync::Once;
pub fn tables() -> Option<Smbios<'static>> {
    None
}
