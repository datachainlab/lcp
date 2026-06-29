use crate::{prelude::*, TypeError};
use core::fmt::Display;
use core::mem;
use core::ops::Deref;
use sgx_types::{
    metadata::{
        dir_index_t, metadata_t, GROUP_FLAG, LAYOUT_ID_TCS_DYN, PAGE_ATTR_EADD, PAGE_ATTR_POST_ADD,
        SI_FLAGS_TCS, TCS_POLICY_BIND, TCS_POLICY_UNBIND,
    },
    sgx_measurement_t, SGX_HASH_SIZE,
};

/// MRENCLAVE is a measurement of the enclave
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Mrenclave(pub [u8; SGX_HASH_SIZE]);

impl Deref for Mrenclave {
    type Target = [u8; SGX_HASH_SIZE];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Mrenclave {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}

impl From<sgx_measurement_t> for Mrenclave {
    fn from(measurement: sgx_measurement_t) -> Self {
        Self(measurement.m)
    }
}

impl From<Mrenclave> for sgx_measurement_t {
    fn from(mrenclave: Mrenclave) -> Self {
        sgx_measurement_t { m: mrenclave.0 }
    }
}

impl From<[u8; SGX_HASH_SIZE]> for Mrenclave {
    fn from(bytes: [u8; SGX_HASH_SIZE]) -> Self {
        Self(bytes)
    }
}

impl TryFrom<Vec<u8>> for Mrenclave {
    type Error = TypeError;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != SGX_HASH_SIZE {
            return Err(TypeError::mrenclave_bytes_conversion(value));
        }
        let mut bytes = [0u8; SGX_HASH_SIZE];
        bytes.copy_from_slice(&value);
        Ok(Self(bytes))
    }
}

impl Mrenclave {
    pub fn to_hex_string(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }
    pub fn from_hex_string(s: &str) -> Result<Self, TypeError> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let mut bytes = [0u8; SGX_HASH_SIZE];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Self(bytes))
    }
}

/// EnclaveMetadata is the metadata of an enclave
pub struct EnclaveMetadata(metadata_t);

impl Deref for EnclaveMetadata {
    type Target = metadata_t;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<metadata_t> for EnclaveMetadata {
    fn from(metadata: metadata_t) -> Self {
        Self(metadata)
    }
}

impl From<EnclaveMetadata> for metadata_t {
    fn from(metadata: EnclaveMetadata) -> Self {
        metadata.0
    }
}

impl EnclaveMetadata {
    /// Get the MRENCLAVE of the enclave from the metadata
    pub fn mrenclave(&self) -> Mrenclave {
        self.enclave_css.body.enclave_hash.m.into()
    }

    /// Get the enclave TCS policy from the metadata.
    pub fn tcs_policy(&self) -> u32 {
        self.tcs_policy
    }

    /// Returns true when the enclave metadata declares `TCSPolicy=BIND`.
    pub fn tcs_policy_is_bind(&self) -> bool {
        self.tcs_policy() == TCS_POLICY_BIND
    }

    /// Returns true when the enclave metadata declares `TCSPolicy=UNBIND`.
    pub fn tcs_policy_is_unbind(&self) -> bool {
        self.tcs_policy() == TCS_POLICY_UNBIND
    }

    /// Derive the configured TCS count from the enclave metadata layout table.
    ///
    /// `sgx_get_metadata` does not expose the original `Enclave.config.xml`
    /// `<TCSNum>` directly. The signed metadata does contain the final layout
    /// table, so this method mirrors the SGX runtime's TCS counting logic and
    /// returns the number of TCS entries available to the enclave.
    pub fn tcs_num(&self) -> Option<usize> {
        let layout_dir = self.dirs[dir_index_t::DIR_LAYOUT as usize];
        let offset = layout_dir.offset as usize;
        let size = layout_dir.size as usize;
        let metadata = unsafe {
            core::slice::from_raw_parts(
                (&self.0 as *const metadata_t).cast::<u8>(),
                mem::size_of::<metadata_t>(),
            )
        };
        let end = offset.checked_add(size)?;
        if size == 0 || end > metadata.len() {
            return None;
        }

        let layout = &metadata[offset..end];
        let entry_count = layout.len() / LAYOUT_ENTRY_SIZE;
        if entry_count == 0 {
            return None;
        }

        let count = count_tcs_entries(layout, 0, entry_count);
        (count > 0).then_some(count)
    }
}

const LAYOUT_ENTRY_SIZE: usize = 32;

fn count_tcs_entries(layout: &[u8], start: usize, end: usize) -> usize {
    let mut count = 0usize;
    for index in start..end {
        let offset = index * LAYOUT_ENTRY_SIZE;
        let Some(id) = read_u16(layout, offset) else {
            break;
        };
        let id = id as u32;

        if (id & GROUP_FLAG) != 0 {
            let Some(entry_count) = read_u16(layout, offset + 2).map(usize::from) else {
                continue;
            };
            let Some(load_times) = read_u32(layout, offset + 4).map(|v| v as usize) else {
                continue;
            };
            let preceding_entries = index - start;
            if entry_count == 0 || entry_count > preceding_entries {
                continue;
            }
            let group_start = index - entry_count;
            let group_count = count_tcs_entries(layout, group_start, index);
            count = count.saturating_add(group_count.saturating_mul(load_times));
            continue;
        }

        let Some(attributes) = read_u16(layout, offset + 2) else {
            continue;
        };
        if (attributes & PAGE_ATTR_EADD) != 0 {
            let Some(content_offset) = read_u32(layout, offset + 20) else {
                continue;
            };
            let Some(si_flags) = read_u64(layout, offset + 24) else {
                continue;
            };
            if content_offset != 0 && si_flags == SI_FLAGS_TCS {
                count = count.saturating_add(1);
                continue;
            }
        }
        if (attributes & PAGE_ATTR_POST_ADD) != 0 && id == LAYOUT_ID_TCS_DYN {
            count = count.saturating_add(1);
        }
    }
    count
}

fn read_u16(input: &[u8], offset: usize) -> Option<u16> {
    let bytes = input.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_le_bytes(bytes.try_into().ok()?))
}

fn read_u32(input: &[u8], offset: usize) -> Option<u32> {
    let bytes = input.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

fn read_u64(input: &[u8], offset: usize) -> Option<u64> {
    let bytes = input.get(offset..offset.checked_add(8)?)?;
    Some(u64::from_le_bytes(bytes.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::MaybeUninit;
    use sgx_types::metadata::{LAYOUT_ID_TCS, LAYOUT_ID_THREAD_GROUP};

    fn empty_metadata() -> metadata_t {
        unsafe { MaybeUninit::zeroed().assume_init() }
    }

    fn metadata_data_offset() -> usize {
        let metadata = MaybeUninit::<metadata_t>::uninit();
        let base = metadata.as_ptr() as usize;
        let data = unsafe { core::ptr::addr_of!((*metadata.as_ptr()).data) as usize };
        data - base
    }

    fn write_u16(output: &mut [u8], offset: usize, value: u16) {
        output[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(output: &mut [u8], offset: usize, value: u32) {
        output[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(output: &mut [u8], offset: usize, value: u64) {
        output[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_tcs_entry(output: &mut [u8], index: usize) {
        let offset = index * LAYOUT_ENTRY_SIZE;
        write_u16(output, offset, LAYOUT_ID_TCS as u16);
        write_u16(output, offset + 2, PAGE_ATTR_EADD as u16);
        write_u32(output, offset + 20, 1);
        write_u64(output, offset + 24, SI_FLAGS_TCS);
    }

    fn write_thread_group(output: &mut [u8], index: usize, entry_count: u16, load_times: u32) {
        let offset = index * LAYOUT_ENTRY_SIZE;
        write_u16(output, offset, LAYOUT_ID_THREAD_GROUP as u16);
        write_u16(output, offset + 2, entry_count);
        write_u32(output, offset + 4, load_times);
    }

    fn metadata_with_layout(layout: &[u8]) -> EnclaveMetadata {
        let mut metadata = empty_metadata();
        metadata.dirs[dir_index_t::DIR_LAYOUT as usize].offset = metadata_data_offset() as u32;
        metadata.dirs[dir_index_t::DIR_LAYOUT as usize].size = layout.len() as u32;
        metadata.data[..layout.len()].copy_from_slice(layout);
        metadata.into()
    }

    #[test]
    fn tcs_num_counts_tcs_layout_entries() {
        let mut layout = [0u8; LAYOUT_ENTRY_SIZE * 2];
        write_tcs_entry(&mut layout, 0);
        write_tcs_entry(&mut layout, 1);

        assert_eq!(metadata_with_layout(&layout).tcs_num(), Some(2));
    }

    #[test]
    fn tcs_num_counts_group_repetitions_like_sgx_runtime() {
        let mut layout = [0u8; LAYOUT_ENTRY_SIZE * 2];
        write_tcs_entry(&mut layout, 0);
        write_thread_group(&mut layout, 1, 1, 3);

        assert_eq!(metadata_with_layout(&layout).tcs_num(), Some(4));
    }

    #[test]
    fn tcs_num_returns_none_when_layout_is_absent() {
        assert_eq!(EnclaveMetadata::from(empty_metadata()).tcs_num(), None);
    }
}
