use crate::audit::merkle::{keccak256, verify_proof, MerkleTree};

#[test]
fn test_merkle_tree_root_consistency() {
    let leaves: Vec<[u8; 32]> = (0u8..4).map(|i| keccak256(&[i])).collect();
    let tree = MerkleTree::new(leaves.clone());
    let tree2 = MerkleTree::new(leaves);
    assert_eq!(tree.root, tree2.root);
}

#[test]
fn test_merkle_proof_verification() {
    let leaves: Vec<[u8; 32]> = (0u8..4).map(|i| keccak256(&[i])).collect();
    let tree = MerkleTree::new(leaves);

    let (proof, leaf) = tree.generate_proof(2).unwrap();
    assert!(verify_proof(tree.root, leaf, &proof));
}

#[test]
fn test_merkle_tampered_leaf_fails() {
    let leaves: Vec<[u8; 32]> = (0u8..4).map(|i| keccak256(&[i])).collect();
    let tree = MerkleTree::new(leaves);

    let (proof, _) = tree.generate_proof(1).unwrap();
    let fake_leaf = keccak256(b"fake");
    assert!(!verify_proof(tree.root, fake_leaf, &proof));
}
