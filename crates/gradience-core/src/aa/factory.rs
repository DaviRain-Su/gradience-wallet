use alloy::primitives::{Address, B256, U256, keccak256};

/// Compute the counterfactual CREATE2 address for an ERC-4337 SimpleAccount.
///
/// # Arguments
/// - `factory`        – the `SimpleAccountFactory` contract address.
/// - `salt`           – the salt passed to `createAccount(owner, salt)`.
/// - `init_code_hash` – `keccak256` of the factory call data that deploys the account.
///
/// # Example
/// ```ignore
/// let factory = "0x...".parse().unwrap();
/// let salt = U256::ZERO;
/// let init_code_hash = keccak256(create_account_calldata(owner, salt));
/// let addr = simple_account_address(factory, salt, init_code_hash);
/// ```
pub fn simple_account_address(factory: Address, salt: U256, init_code_hash: B256) -> Address {
    let mut buf = Vec::with_capacity(1 + 20 + 32 + 32);
    buf.push(0xff);
    buf.extend_from_slice(factory.as_slice());

    let salt_bytes: [u8; 32] = salt.to_be_bytes();
    buf.extend_from_slice(&salt_bytes);

    buf.extend_from_slice(init_code_hash.as_slice());
    let hash = keccak256(&buf);
    Address::from_slice(&hash.as_slice()[12..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create2_deterministic() {
        let factory = Address::ZERO;
        let salt = U256::ZERO;
        let init_hash = B256::ZERO;
        let addr1 = simple_account_address(factory, salt, init_hash);
        let addr2 = simple_account_address(factory, salt, init_hash);
        assert_eq!(addr1, addr2);
    }
}
