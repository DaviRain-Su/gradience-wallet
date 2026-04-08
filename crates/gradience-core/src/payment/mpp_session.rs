//! MPP Session layer built on top of MppEscrow.sol.
//!
//! This provides the client-side escrow + voucher signing for `intent="session"`
//! even though the official `mpp` crate currently only ships `intent="charge"`.

use alloy::primitives::{Address, FixedBytes, U256, keccak256};
use alloy::signers::{Signer, local::PrivateKeySigner};
use crate::error::GradienceError;
use crate::rpc::evm::EvmRpcClient;

/// EIP-191 Ethereum Signed Message wrapper over a raw 32-byte digest.
pub fn eth_message_hash(digest: FixedBytes<32>) -> FixedBytes<32> {
    let prefix = format!("\x19Ethereum Signed Message:\n32{}", hex::encode(digest.as_slice()));
    keccak256(prefix.as_bytes())
}

/// Sign a session voucher: keccak256(sessionId || amount) as an Ethereum signed message.
pub async fn sign_voucher(
    session_id: FixedBytes<32>,
    amount: u128,
    signer: &PrivateKeySigner,
) -> Result<Vec<u8>, GradienceError> {
    let mut packed = session_id.to_vec();
    packed.extend_from_slice(&U256::from(amount).to_be_bytes_vec());
    let digest = keccak256(&packed);
    let eth_hash = eth_message_hash(digest);
    let sig = signer.sign_hash(&eth_hash).await.map_err(|e| {
        GradienceError::InvalidCredential(format!("sign voucher failed: {}", e))
    })?;
    Ok(sig.as_bytes().to_vec())
}

/// Session state as read from the escrow contract.
#[derive(Debug, Clone)]
pub struct SessionState {
    pub sender: Address,
    pub recipient: Address,
    pub deposit: U256,
    pub spent: U256,
    pub expires_at: u64,
    pub closed: bool,
}

/// Minimal Solidity interface for MppEscrow.sol.
alloy::sol! {
    contract MppEscrow {
        function openSession(bytes32 sessionId, address recipient, uint256 expiresAt) external payable;
        function redeemVoucher(bytes32 sessionId, uint256 amount, bytes calldata signature) external;
        function closeSession(bytes32 sessionId) external;
        function sessions(bytes32 sessionId) external view returns (address sender, address recipient, uint256 deposit, uint256 spent, uint256 expiresAt, bool closed);
    }
}

/// High-level manager for escrow-based payment sessions.
///
/// Note: transaction broadcasting is left to the caller (via ows_lib or raw RPC)
/// so this module stays dependency-light and compatible with Gradience's
/// existing wallet/signing stack.
pub struct MppSessionManager {
    rpc_url: String,
    contract_address: Address,
}

impl MppSessionManager {
    pub fn new(rpc_url: impl Into<String>, contract_address: Address) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            contract_address,
        }
    }

    /// Build the hex-encoded calldata for `openSession`.
    pub fn build_open_session_calldata(
        &self,
        session_id: FixedBytes<32>,
        recipient: Address,
        expires_at: u64,
    ) -> String {
        let call = MppEscrow::openSessionCall {
            sessionId,
            recipient,
            expiresAt: U256::from(expires_at),
        };
        format!("0x{}", hex::encode(call.abi_encode()))
    }

    /// Build the hex-encoded calldata for `closeSession`.
    pub fn build_close_session_calldata(
        &self,
        session_id: FixedBytes<32>,
    ) -> String {
        let call = MppEscrow::closeSessionCall { sessionId: session_id };
        format!("0x{}", hex::encode(call.abi_encode()))
    }

    /// Build the hex-encoded calldata for `redeemVoucher`.
    pub fn build_redeem_voucher_calldata(
        &self,
        session_id: FixedBytes<32>,
        amount: u128,
        signature: &[u8],
    ) -> String {
        let call = MppEscrow::redeemVoucherCall {
            sessionId: session_id,
            amount: U256::from(amount),
            signature: signature.to_vec().into(),
        };
        format!("0x{}", hex::encode(call.abi_encode()))
    }

    /// Read session state from the contract via eth_call.
    pub async fn get_session(
        &self,
        session_id: FixedBytes<32>,
    ) -> Result<SessionState, GradienceError> {
        let client = EvmRpcClient::new("evm", &self.rpc_url)?;
        let call = MppEscrow::sessionsCall { sessionId: session_id };
        let data = format!("0x{}", hex::encode(call.abi_encode()));
        let resp = client.eth_call(&self.contract_address.to_string(),
            &data,
        ).await?;

        let hex = resp.strip_prefix("0x").unwrap_or(&resp);
        let bytes = hex::decode(hex).map_err(|e| {
            GradienceError::Blockchain(format!("decode eth_call response: {}", e))
        })?;

        // Decode ABI: (address,address,uint256,uint256,uint256,bool)
        // 6 items = 6 * 32 = 192 bytes of response starting at offset 0
        if bytes.len() < 192 {
            return Err(GradienceError::Blockchain("short sessions response".into()));
        }

        let sender = alloy::primitives::Address::from_slice(&bytes[12..32]);
        let recipient = alloy::primitives::Address::from_slice(&bytes[44..64]);
        let deposit = U256::from_be_slice(&bytes[64..96]);
        let spent = U256::from_be_slice(&bytes[96..128]);
        let expires_at = U256::from_be_slice(&bytes[128..160]);
        let closed = bytes[191] != 0;

        Ok(SessionState {
            sender,
            recipient,
            deposit,
            spent,
            expires_at: expires_at.try_into().unwrap_or(0u64),
            closed,
        })
    }
}
