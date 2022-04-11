#[macro_export]
macro_rules! validate_const_ptr {
    ($ptr:expr, $ptr_len:expr, $ret_val:expr $(,)?) => {{
        if let Err(_e) = $crate::pointers::validate_const_ptr($ptr, $ptr_len) {
            ::log::error!("Tried to access data outside enclave memory!");
            return $ret_val;
        }
    }};
}

#[macro_export]
macro_rules! validate_mut_ptr {
    ($ptr:expr, $ptr_len:expr, $ret_val:expr $(,)?) => {{
        if let Err(_e) = $crate::pointers::validate_mut_ptr($ptr, $ptr_len) {
            ::log::error!("Tried to access data outside enclave memory!");
            return $ret_val;
        }
    }};
}
