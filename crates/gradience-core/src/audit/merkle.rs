use sha3::{Keccak256, Digest};
use std::convert::TryInto;

#[derive(Debug, Clone)]
pub struct MerkleTree {
    pub root: [u8; 32],
    pub leaves: Vec<[u8; 32]>,
    pub layers: Vec<Vec<[u8; 32]>>,
}

impl MerkleTree {
    pub fn new(leaves: Vec<[u8; 32]>) -> Self {
        if leaves.is_empty() {
            return Self {
                root: keccak256(b"empty"),
                leaves: vec![],
                layers: vec![],
            };
        }
        let mut layers: Vec<Vec<[u8; 32]>> = vec![leaves.clone()];
        let mut current = leaves.clone();
        while current.len() > 1 {
            let mut next = Vec::new();
            for chunk in current.chunks(2) {
                let left = chunk[0];
                let right = if chunk.len() == 2 { chunk[1] } else { left };
                next.push(hash_pair(&left, &right));
            }
            layers.push(next.clone());
            current = next;
        }
        let root = current[0];
        Self { root, leaves, layers }
    }

    pub fn generate_proof(&self,
        index: usize,
    ) -> Option<(Vec<[u8; 32]>, [u8; 32])> {
        if index >= self.leaves.len() {
            return None;
        }
        let mut proof = Vec::new();
        let mut idx = index;
        for layer in &self.layers {
            if layer.len() <= 1 {
                break;
            }
            let sibling = if idx.is_multiple_of(2) {
                if idx + 1 < layer.len() { layer[idx + 1] } else { layer[idx] }
            } else {
                layer[idx - 1]
            };
            proof.push(sibling);
            idx /= 2;
        }
        Some((proof, self.leaves[index]))
    }
}

pub fn verify_proof(root: [u8; 32], leaf: [u8; 32], proof: &[[u8; 32]]) -> bool {
    let mut current = leaf;
    for sibling in proof {
        current = hash_pair(&current, sibling);
    }
    current == root
}

fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    if left <= right {
        hasher.update(left);
        hasher.update(right);
    } else {
        hasher.update(right);
        hasher.update(left);
    }
    hasher.finalize().as_slice().try_into().unwrap()
}

pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().as_slice().try_into().unwrap()
}
