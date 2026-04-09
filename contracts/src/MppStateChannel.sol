// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/**
 * @title MppStateChannel — State channel for MPP micro-payments
 * @notice Enables off-chain signed state updates with on-chain settlement + challenge period.
 */
contract MppStateChannel {
    struct Channel {
        address payer;
        address payee;
        uint256 deposit;
        uint256 settledAmount;
        uint256 challengePeriod;
        uint256 expiresAt;
        bool closed;
        uint256 challengeEnd;
        uint256 latestNonce;
    }

    mapping(bytes32 => Channel) public channels;

    event ChannelOpened(
        bytes32 indexed channelId,
        address indexed payer,
        address indexed payee,
        uint256 deposit,
        uint256 challengePeriod,
        uint256 expiresAt
    );

    event SettlementInitiated(
        bytes32 indexed channelId,
        uint256 nonce,
        uint256 amount,
        uint256 challengeEnd
    );

    event Challenged(
        bytes32 indexed channelId,
        uint256 nonce,
        uint256 amount,
        uint256 challengeEnd
    );

    event SettlementConfirmed(
        bytes32 indexed channelId,
        uint256 amount,
        uint256 refund
    );

    event ForceClosed(
        bytes32 indexed channelId,
        uint256 refund
    );

    function openChannel(
        bytes32 channelId,
        address payee,
        uint256 challengePeriod,
        uint256 expiresAt
    ) external payable {
        require(channels[channelId].payer == address(0), "Channel exists");
        require(msg.value > 0, "Zero deposit");
        require(payee != address(0), "Invalid payee");
        require(payee != msg.sender, "Payer is payee");
        require(challengePeriod > 0, "Zero challenge period");
        require(expiresAt > block.timestamp, "Invalid expiry");

        channels[channelId] = Channel({
            payer: msg.sender,
            payee: payee,
            deposit: msg.value,
            settledAmount: 0,
            challengePeriod: challengePeriod,
            expiresAt: expiresAt,
            closed: false,
            challengeEnd: 0,
            latestNonce: 0
        });

        emit ChannelOpened(
            channelId,
            msg.sender,
            payee,
            msg.value,
            challengePeriod,
            expiresAt
        );
    }

    function initiateSettlement(
        bytes32 channelId,
        uint256 nonce,
        uint256 amount,
        bytes calldata signature
    ) external {
        Channel storage c = channels[channelId];
        require(!c.closed, "Channel closed");
        require(msg.sender == c.payee, "Only payee");
        require(nonce > c.latestNonce, "Nonce not increasing");
        require(amount <= c.deposit, "Amount exceeds deposit");
        require(block.timestamp < c.expiresAt, "Channel expired");

        _verifySignature(channelId, nonce, amount, signature, c.payer);

        c.latestNonce = nonce;
        c.settledAmount = amount;
        c.challengeEnd = block.timestamp + c.challengePeriod;

        emit SettlementInitiated(channelId, nonce, amount, c.challengeEnd);
    }

    function challenge(
        bytes32 channelId,
        uint256 nonce,
        uint256 amount,
        bytes calldata signature
    ) external {
        Channel storage c = channels[channelId];
        require(!c.closed, "Channel closed");
        require(msg.sender == c.payer, "Only payer");
        require(block.timestamp < c.challengeEnd, "Challenge period over");
        require(nonce > c.latestNonce, "Nonce not higher");
        require(amount <= c.deposit, "Amount exceeds deposit");

        _verifySignature(channelId, nonce, amount, signature, c.payer);

        c.latestNonce = nonce;
        c.settledAmount = amount;
        c.challengeEnd = block.timestamp + c.challengePeriod;

        emit Challenged(channelId, nonce, amount, c.challengeEnd);
    }

    function confirmSettlement(bytes32 channelId) external {
        Channel storage c = channels[channelId];
        require(!c.closed, "Channel closed");
        require(c.challengeEnd > 0, "No settlement pending");
        require(block.timestamp >= c.challengeEnd, "Challenge active");

        c.closed = true;
        uint256 payeeAmount = c.settledAmount;
        uint256 refund = c.deposit - payeeAmount;

        if (payeeAmount > 0) {
            payable(c.payee).transfer(payeeAmount);
        }
        if (refund > 0) {
            payable(c.payer).transfer(refund);
        }

        emit SettlementConfirmed(channelId, payeeAmount, refund);
    }

    function forceClose(bytes32 channelId) external {
        Channel storage c = channels[channelId];
        require(!c.closed, "Channel closed");
        require(msg.sender == c.payer, "Only payer");
        require(block.timestamp >= c.expiresAt, "Not expired");

        c.closed = true;
        uint256 refund = c.deposit - c.settledAmount;

        if (refund > 0) {
            payable(c.payer).transfer(refund);
        }

        emit ForceClosed(channelId, refund);
    }

    function _verifySignature(
        bytes32 channelId,
        uint256 nonce,
        uint256 amount,
        bytes calldata signature,
        address expectedSigner
    ) internal pure {
        bytes32 digest = keccak256(
            abi.encodePacked(channelId, nonce, amount)
        );
        bytes32 ethHash = keccak256(
            abi.encodePacked("\x19Ethereum Signed Message:\n32", digest)
        );
        address signer = recoverSigner(ethHash, signature);
        require(signer == expectedSigner, "Invalid signature");
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
