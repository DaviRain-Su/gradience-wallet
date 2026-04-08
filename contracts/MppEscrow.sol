// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/**
 * @title MppEscrow — Machine Payments Protocol escrow for Agent micro-payments
 * @notice Enables off-chain signed vouchers to be redeemed on-chain within a session budget.
 */
contract MppEscrow {
    struct Session {
        address sender;
        address recipient;
        uint256 deposit;
        uint256 spent;
        uint256 expiresAt;
        bool closed;
    }

    mapping(bytes32 => Session) public sessions;

    event SessionOpened(
        bytes32 indexed sessionId,
        address indexed sender,
        address indexed recipient,
        uint256 deposit,
        uint256 expiresAt
    );

    event VoucherRedeemed(
        bytes32 indexed sessionId,
        uint256 amount,
        uint256 remaining
    );

    event SessionClosed(
        bytes32 indexed sessionId,
        uint256 refund
    );

    function openSession(
        bytes32 sessionId,
        address recipient,
        uint256 expiresAt
    ) external payable {
        require(sessions[sessionId].sender == address(0), "Session exists");
        require(msg.value > 0, "Zero deposit");
        require(recipient != address(0), "Invalid recipient");
        require(expiresAt > block.timestamp, "Invalid expiry");

        sessions[sessionId] = Session({
            sender: msg.sender,
            recipient: recipient,
            deposit: msg.value,
            spent: 0,
            expiresAt: expiresAt,
            closed: false
        });

        emit SessionOpened(sessionId, msg.sender, recipient, msg.value, expiresAt);
    }

    function redeemVoucher(
        bytes32 sessionId,
        uint256 amount,
        bytes calldata signature
    ) external {
        Session storage s = sessions[sessionId];
        require(!s.closed, "Session closed");
        require(block.timestamp < s.expiresAt, "Session expired");
        require(s.spent + amount <= s.deposit, "Insufficient deposit");

        // Verify ECDSA signature over (sessionId || amount)
        bytes32 digest = keccak256(abi.encodePacked(sessionId, amount));
        bytes32 ethHash = keccak256(
            abi.encodePacked("\x19Ethereum Signed Message:\n32", digest)
        );
        address signer = recoverSigner(ethHash, signature);
        require(signer == s.sender, "Invalid signature");

        s.spent += amount;
        payable(s.recipient).transfer(amount);

        emit VoucherRedeemed(sessionId, amount, s.deposit - s.spent);
    }

    function closeSession(bytes32 sessionId) external {
        Session storage s = sessions[sessionId];
        require(!s.closed, "Already closed");
        require(
            msg.sender == s.sender || block.timestamp >= s.expiresAt,
            "Unauthorized"
        );

        s.closed = true;
        uint256 refund = s.deposit - s.spent;
        if (refund > 0) {
            payable(s.sender).transfer(refund);
        }

        emit SessionClosed(sessionId, refund);
    }

    function recoverSigner(bytes32 ethHash, bytes calldata sig)
        internal
        pure
        returns (address)
    {
        require(sig.length == 65, "Bad signature length");

        bytes32 r;
        bytes32 s_;
        uint8 v;

        assembly {
            r := calldataload(add(sig.offset, 32))
            s_ := calldataload(add(sig.offset, 64))
            v := byte(0, calldataload(add(sig.offset, 96)))
        }

        if (v < 27) v += 27;
        return ecrecover(ethHash, v, r, s_);
    }
}
