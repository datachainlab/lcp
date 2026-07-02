use crate::prelude::*;
use core::fmt;
use crypto::{Address, EnclavePublicKey, SealedEnclaveKey};
use lcp_types::BytesTransmuter;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sgx_types::{sgx_report_t, sgx_target_info_t};

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveManageCommand {
    GenerateEnclaveKey(GenerateEnclaveKeyInput),
    RuntimeInfo,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GenerateEnclaveKeyInput {
    #[serde_as(as = "BytesTransmuter<sgx_target_info_t>")]
    pub target_info: sgx_target_info_t,
    pub operator: Option<Address>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveManageResponse {
    GenerateEnclaveKey(GenerateEnclaveKeyResponse),
    RuntimeInfo(EnclaveRuntimeInfo),
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateEnclaveKeyResponse {
    pub pub_key: EnclavePublicKey,
    pub sealed_ek: SealedEnclaveKey,
    #[serde_as(as = "BytesTransmuter<sgx_report_t>")]
    pub report: sgx_report_t,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnclaveThreadPolicy {
    Bound,
    Unbound,
}

impl fmt::Display for EnclaveThreadPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bound => f.write_str("BIND"),
            Self::Unbound => f.write_str("UNBIND"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnclaveRuntimeInfo {
    pub thread_policy: EnclaveThreadPolicy,
    pub static_tcs_num: u32,
    pub eremove_tcs_num: u32,
    pub dyn_tcs_num: u32,
    pub tcs_max_num: u32,
    pub edmm_supported: bool,
}

impl EnclaveRuntimeInfo {
    pub fn effective_tcs_limit(&self) -> usize {
        if self.edmm_supported || self.static_tcs_num == 0 {
            self.tcs_max_num as usize
        } else {
            self.static_tcs_num as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_info(
        static_tcs_num: u32,
        tcs_max_num: u32,
        edmm_supported: bool,
    ) -> EnclaveRuntimeInfo {
        EnclaveRuntimeInfo {
            thread_policy: EnclaveThreadPolicy::Bound,
            static_tcs_num,
            eremove_tcs_num: 0,
            dyn_tcs_num: 0,
            tcs_max_num,
            edmm_supported,
        }
    }

    #[test]
    fn effective_tcs_limit_uses_static_tcs_for_sgx1_when_reported() {
        assert_eq!(runtime_info(8, 16, false).effective_tcs_limit(), 8);
    }

    #[test]
    fn effective_tcs_limit_falls_back_to_max_when_static_tcs_is_zero() {
        assert_eq!(runtime_info(0, 8, false).effective_tcs_limit(), 8);
    }

    #[test]
    fn effective_tcs_limit_uses_max_for_edmm() {
        assert_eq!(runtime_info(4, 16, true).effective_tcs_limit(), 16);
    }
}
