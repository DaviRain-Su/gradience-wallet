// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/**
 * @title AuditAnchor — Gradience Wallet audit log Merkle root anchoring
 * @notice Deployed on HashKey Chain for tamper-proof audit proofs
 */
contract AuditAnchor {
    struct Anchor {
        bytes32 root;
        bytes32 prevRoot;
        uint256 logStartIndex;
        uint256 logEndIndex;
        uint256 leafCount;
        uint256 timestamp;
        address submittedBy;
    }

    mapping(bytes32 => Anchor) public anchors;
    mapping(bytes32 => bool) public isAnchored;
    bytes32 public latestRoot;

    event Anchored(
        bytes32 indexed root,
        bytes32 indexed prevRoot,
        uint256 logStartIndex,
        uint256 logEndIndex,
        uint256 leafCount,
        uint256 timestamp,
        address indexed submittedBy
    );

    function anchor(
        bytes32 root,
        bytes32 prevRoot,
        uint256 logStartIndex,
        uint256 logEndIndex,
        uint256 leafCount
    ) external {
        require(!isAnchored[root], "Root already anchored");
        require(root != bytes32(0), "Invalid root");

        anchors[root] = Anchor({
            root: root,
            prevRoot: prevRoot,
            logStartIndex: logStartIndex,
            logEndIndex: logEndIndex,
            leafCount: leafCount,
            timestamp: block.timestamp,
            submittedBy: msg.sender
        });

        isAnchored[root] = true;
        latestRoot = root;

        emit Anchored(root, prevRoot, logStartIndex, logEndIndex, leafCount, block.timestamp, msg.sender);
    }

    function verifyProof(
        bytes32 root,
        bytes32 leaf,
        bytes32[] calldata proof
    ) external pure returns (bool) {
        bytes32 computedHash = leaf;
        for (uint256 i = 0; i < proof.length; i++) {
            bytes32 proofElement = proof[i];
            if (computedHash <= proofElement) {
                computedHash = keccak256(abi.encodePacked(computedHash, proofElement));
            } else {
                computedHash = keccak256(abi.encodePacked(proofElement, computedHash));
            }
        }
        return computedHash == root;
    }

    function getLatestAnchor() external view returns (Anchor memory) {
        return anchors[latestRoot];
    }

    function verifyChainIntegrity(bytes32 root) external view returns (bool) {
        if (!isAnchored[root]) return false;
        bytes32 current = root;
        while (current != bytes32(0)) {
            Anchor memory a = anchors[current];
            if (a.prevRoot != bytes32(0) && !isAnchored[a.prevRoot]) {
                return false;
            }
            current = a.prevRoot;
        }
        return true;
    }
}
