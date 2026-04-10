use alloy::primitives::{Address, B256, Bytes, U256, keccak256};
use alloy::signers::Signer;
use alloy::sol_types::SolValue;
use alloy_rpc_types_eth::erc4337::UserOperation;

pub struct UserOpBuilder;

impl UserOpBuilder {
    /// Build a basic V0.6 UserOperation.
    pub fn new_v06(
        sender: Address,
        nonce: U256,
        init_code: Bytes,
        call_data: Bytes,
        max_fee_per_gas: U256,
        max_priority_fee_per_gas: U256,
    ) -> UserOperation {
        UserOperation {
            sender,
            nonce,
            init_code,
            call_data,
            call_gas_limit: U256::from(100_000),
            verification_gas_limit: U256::from(100_000),
            pre_verification_gas: U256::from(50_000),
            max_fee_per_gas,
            max_priority_fee_per_gas,
            paymaster_and_data: Bytes::new(),
            signature: Bytes::new(),
        }
    }

    /// Compute the EIP-4337 V0.6 userOp hash.
    ///
    /// Formula:
    /// `keccak256( abi.encode( keccak256(pack), entryPoint, chainId ) )`
    /// where `pack` is the ABI-encoded tuple of all operation fields
    /// (with `initCode`, `callData`, and `paymasterAndData` replaced by
    /// their `keccak256` hashes).
    pub fn hash_v06(op: &UserOperation, entry_point: Address, chain_id: u64) -> B256 {
        let pack = (
            op.sender,
            op.nonce,
            keccak256(&op.init_code),
            keccak256(&op.call_data),
            op.call_gas_limit,
            op.verification_gas_limit,
            op.pre_verification_gas,
            op.max_fee_per_gas,
            op.max_priority_fee_per_gas,
            keccak256(&op.paymaster_and_data),
        )
            .abi_encode();

        let mut outer = Vec::new();
        outer.extend_from_slice(&keccak256(&pack).abi_encode());
        outer.extend_from_slice(&entry_point.abi_encode());
        outer.extend_from_slice(&U256::from(chain_id).abi_encode());
        keccak256(&outer)
    }

    /// Sign the V0.6 userOp hash and return a new UserOperation with the signature set.
    pub async fn sign_v06(
        mut op: UserOperation,
        signer: &impl Signer,
        entry_point: Address,
        chain_id: u64,
    ) -> anyhow::Result<UserOperation> {
        let hash = Self::hash_v06(&op, entry_point, chain_id);
        let sig = signer.sign_hash(&hash).await?;
        op.signature = Bytes::from(sig.as_bytes().to_vec());
        Ok(op)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_v06_deterministic() {
        let op = UserOpBuilder::new_v06(
            Address::ZERO,
            U256::ZERO,
            Bytes::new(),
            Bytes::new(),
            U256::ZERO,
            U256::ZERO,
        );
        let h1 = UserOpBuilder::hash_v06(&op, Address::ZERO, 1);
        let h2 = UserOpBuilder::hash_v06(&op, Address::ZERO, 1);
        assert_eq!(h1, h2);
        assert_ne!(h1, B256::ZERO);
    }
}
