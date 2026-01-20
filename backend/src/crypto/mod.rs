pub mod encryption;
pub mod ethereum;
pub mod password;
pub mod zcash;

pub use encryption::{decrypt, encrypt};
pub use ethereum::{generate_ethereum_wallet, import_ethereum_wallet};
pub use zcash::{generate_zcash_wallet, import_zcash_wallet};
