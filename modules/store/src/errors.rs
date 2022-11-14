use crate::prelude::*;
use crate::TxId;
use flex_error::*;

pub type Result<T> = core::result::Result<T, Error>;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    Error {
        TxIdOverflow
        |_| "tx_id is overflow",

        TxIdNotFound { tx_id: TxId }
        |e| { format_args!("TxId not found: {}", e.tx_id) },

        BeginTx { descr: String }
        |e| { format_args!("Begin transaction error: {}", e.descr) },

        RollbackTx { descr: String }
        |e| { format_args!("Rollback transaction error: {}", e.descr) },

        CommitTx { descr: String }
        |e| { format_args!("Commit transaction error: {}", e.descr) },

        WaitMutex { descr: String }
        |e| { format_args!("Wait mutex error: {}", e.descr) },

        NotSupportedOperation { descr: String }
        |e| { format_args!("The tx doesn't support an operation {}", e.descr) },

        InvalidUpdateKeyLength { length: usize }
        |e| { format_args!("Invalid UpdateKey length: {}", e.length) }
    }
}
