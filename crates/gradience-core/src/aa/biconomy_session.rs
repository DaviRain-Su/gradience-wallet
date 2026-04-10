use alloy::primitives::{keccak256, Address, B256, Bytes, U256};
use alloy::sol_types::{sol, SolCall, SolValue};

sol! {
    contract SessionKeyManagerModule {
        function setMerkleRoot(bytes32 _merkleRoot) external;
    }
}

/// Helpers for Biconomy SessionKeyManagerModule (Merkle Tree version).
pub struct BiconomySession;

impl BiconomySession {
    /// Compute the session leaf hash used by the Merkle Tree.
    ///
    /// Biconomy packs: `validUntil (6 bytes) || validAfter (6 bytes) || sessionValidationModule (20 bytes) || sessionKeyData`
    pub fn leaf_hash(
        valid_until: u64,
        valid_after: u64,
        session_validation_module: Address,
        session_key_data: &Bytes,
    ) -> B256 {
        let mut packed = Vec::with_capacity(6 + 6 + 20 + session_key_data.len());
        // uint48 -> last 6 bytes of U256 BE
        packed.extend_from_slice(&U256::from(valid_until).to_be_bytes::<32>()[26..]);
        packed.extend_from_slice(&U256::from(valid_after).to_be_bytes::<32>()[26..]);
        packed.extend_from_slice(session_validation_module.as_slice());
        packed.extend_from_slice(session_key_data);
        keccak256(&packed)
    }

    /// For a single-leaf tree, the root is the leaf itself and proof is empty.
    pub fn single_leaf_merkle_root(leaf: &B256) -> B256 {
        *leaf
    }

    /// Build the `setMerkleRoot(root)` call data for enabling a session on-chain.
    pub fn build_set_merkle_root_call_data(merkle_root: B256) -> Bytes {
        SessionKeyManagerModule::setMerkleRootCall {
            _merkleRoot: merkle_root,
        }
        .abi_encode()
        .into()
    }

    /// Build the module signature for a session-enabled UserOp.
    ///
    /// Format: `abi.encode(validUntil, validAfter, sessionValidationModule, sessionKeyData, merkleProof, sessionKeySignature)`
    pub fn build_module_signature(
        valid_until: u64,
        valid_after: u64,
        session_validation_module: Address,
        session_key_data: Bytes,
        merkle_proof: Vec<B256>,
        session_key_signature: &[u8],
    ) -> Bytes {
        let proof_array: Vec<alloy::primitives::FixedBytes<32>> =
            merkle_proof.into_iter().collect();
        let sig_bytes = Bytes::from(session_key_signature.to_vec());
        (
            U256::from(valid_until),
            U256::from(valid_after),
            session_validation_module,
            session_key_data,
            proof_array,
            sig_bytes,
        )
            .abi_encode_sequence()
            .into()
    }

    /// Build the final `userOp.signature` used by Biconomy SmartAccount.validateUserOp.
    ///
    /// Format: `abi.encode(moduleSignature, sessionKeyManagerModuleAddress)`
    pub fn build_user_op_signature(module_signature: Bytes, skm_address: Address) -> Bytes {
        (module_signature, skm_address).abi_encode_sequence().into()
    }

    /// Utility: encode ERC20SessionValidationModule sessionKeyData.
    /// Biconomy V2 expects 5 fields:
    /// `abi.encode(address sessionKey, address token, address recipient, uint256 maxAmount, uint256 maxUsage)`
    /// where `maxUsage` packs `userOpSender` in the high 160 bits and `usageCount` in the low 64 bits.
    pub fn encode_erc20_session_key_data(
        session_key: Address,
        token: Address,
        recipient: Address,
        max_amount: U256,
        smart_account: Address,
        max_usage_count: u64,
    ) -> Bytes {
        let max_usage =
            (U256::from_be_bytes(smart_account.into_word().into()) << 64) | U256::from(max_usage_count);
        (session_key, token, recipient, max_amount, max_usage)
            .abi_encode_sequence()
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::address;

    #[test]
    fn test_leaf_hash_deterministic() {
        let svm = address!("000000D50C68705bd6897B2d17c7de32FB519fDA");
        let session_key = address!("1Be31A94361a391bBaFB2a4CCd704F57dc04d4bb");
        let token = address!("74b7F16337b8972027F6196A1aE9b4dDc0C42b50");
        let recipient = address!("909E30bdBCb728131E3F8d17150eaE740C904649");
        let max = U256::from(1_000_000);

        let smart_account = address!("e7825CD90B7DA8f84049d3f9FC3d2c7D02Ee5989");
        let session_key_data =
            BiconomySession::encode_erc20_session_key_data(session_key, token, recipient, max, smart_account, 1000);
        let h1 = BiconomySession::leaf_hash(0, 0, svm, &session_key_data);
        let h2 = BiconomySession::leaf_hash(0, 0, svm, &session_key_data);
        assert_eq!(h1, h2);
        assert_ne!(h1, B256::ZERO);
    }
}
