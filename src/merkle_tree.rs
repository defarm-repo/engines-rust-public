//! Merkle State Tree (MST) - Core data structures and algorithms
//!
//! This module provides a hierarchical Merkle tree structure for:
//! - Sync verification: Compare circuit/item roots to instantly know if in sync
//! - Proof of inclusion: Prove an event/item exists without revealing others
//! - Efficient diff detection: Find exactly which items/events differ
//! - Light client support: Verify state without downloading all data
//!
//! Tree Structure:
//! ```text
//!                     Circuit Root Hash
//!                            │
//!           ┌────────────────┼────────────────┐
//!           │                │                │
//!      Item 1 Root      Item 2 Root      Item 3 Root
//!           │                │                │
//!     ┌─────┼─────┐    ┌─────┼─────┐         │
//!     │     │     │    │     │     │         │
//!   Ev1   Ev2   Ev3  Ev1   Ev2   Ev3       Ev1
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A node in the Merkle tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    /// BLAKE3 hash (64-char hex string)
    pub hash: String,
    /// Left child (for internal nodes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<Box<MerkleNode>>,
    /// Right child (for internal nodes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<Box<MerkleNode>>,
    /// Leaf data identifier (event_id, dfid) - only for leaf nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_id: Option<String>,
    /// Type of this node
    pub node_type: MerkleNodeType,
}

/// Type of Merkle tree node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MerkleNodeType {
    /// Contains actual data hash (leaf node)
    Leaf,
    /// Hash of children (internal node)
    Internal,
    /// Padding for unbalanced trees
    Empty,
}

/// Proof that a leaf exists in a Merkle tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Hash of the leaf being proved
    pub leaf_hash: String,
    /// Index of the leaf in the tree (0-based)
    pub leaf_index: usize,
    /// Path from leaf to root (sibling hashes)
    pub siblings: Vec<ProofSibling>,
    /// Root hash of the tree
    pub root: String,
}

/// A sibling node in a Merkle proof path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofSibling {
    /// Hash of the sibling node
    pub hash: String,
    /// Position of the sibling relative to the path
    pub position: SiblingPosition,
}

/// Position of a sibling in the Merkle proof
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SiblingPosition {
    /// Sibling is on the left
    Left,
    /// Sibling is on the right
    Right,
}

/// Root summary for quick sync comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleRootSummary {
    /// The root hash
    pub root_hash: String,
    /// Number of leaf nodes
    pub leaf_count: usize,
    /// Height of the tree
    pub tree_height: usize,
    /// When this root was computed
    pub computed_at: DateTime<Utc>,
    /// Type of entity this root represents
    pub entity_type: MerkleEntityType,
    /// ID of the entity (dfid, circuit_id)
    pub entity_id: String,
}

/// Type of entity a Merkle root represents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MerkleEntityType {
    /// Single event hash
    Event,
    /// Root of item's events
    Item,
    /// Root of circuit's items
    Circuit,
}

/// Entry for an item in a circuit's Merkle tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemMerkleEntry {
    /// The item's DFID
    pub dfid: String,
    /// Merkle root of the item's events
    pub merkle_root: String,
    /// Number of events for this item
    pub event_count: usize,
}

/// Direction taken at each node in the path (used for proof generation)
#[derive(Debug, Clone, Copy)]
enum PathDirection {
    Left,
    Right,
}

/// Merkle tree builder and utilities
#[derive(Debug, Clone)]
pub struct MerkleTree {
    /// Root node of the tree
    root: Option<MerkleNode>,
    /// Leaf hashes in order (for proof generation)
    leaves: Vec<String>,
    /// Data IDs corresponding to leaves (optional, for lookup)
    leaf_ids: Vec<Option<String>>,
}

/// Errors that can occur in Merkle tree operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MerkleError {
    /// Tree has no leaves
    EmptyTree,
    /// Leaf index out of bounds
    LeafIndexOutOfBounds { index: usize, max: usize },
    /// Proof verification failed
    ProofVerificationFailed { reason: String },
    /// Storage error
    StorageError(String),
    /// Other error
    Other(String),
}

impl std::fmt::Display for MerkleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MerkleError::EmptyTree => write!(f, "Merkle tree has no leaves"),
            MerkleError::LeafIndexOutOfBounds { index, max } => {
                write!(f, "Leaf index {} out of bounds (max: {})", index, max)
            }
            MerkleError::ProofVerificationFailed { reason } => {
                write!(f, "Proof verification failed: {}", reason)
            }
            MerkleError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            MerkleError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for MerkleError {}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

impl MerkleTree {
    /// Create a new empty Merkle tree
    pub fn new() -> Self {
        MerkleTree {
            root: None,
            leaves: Vec::new(),
            leaf_ids: Vec::new(),
        }
    }

    /// Build a Merkle tree from a list of leaf hashes
    ///
    /// Leaves are sorted before tree construction for deterministic ordering.
    pub fn from_leaves(mut leaves: Vec<String>) -> Self {
        Self::from_leaves_with_ids(leaves.drain(..).map(|h| (h, None)).collect())
    }

    /// Build a Merkle tree from leaves with optional data IDs
    ///
    /// Leaves are sorted by hash before tree construction for deterministic ordering.
    pub fn from_leaves_with_ids(mut leaf_data: Vec<(String, Option<String>)>) -> Self {
        if leaf_data.is_empty() {
            return MerkleTree::new();
        }

        // Sort by hash for deterministic ordering
        leaf_data.sort_by(|a, b| a.0.cmp(&b.0));

        let leaves: Vec<String> = leaf_data.iter().map(|(h, _)| h.clone()).collect();
        let leaf_ids: Vec<Option<String>> = leaf_data.iter().map(|(_, id)| id.clone()).collect();

        // Create leaf nodes
        let mut nodes: Vec<MerkleNode> = leaf_data
            .into_iter()
            .map(|(hash, data_id)| MerkleNode {
                hash,
                left: None,
                right: None,
                data_id,
                node_type: MerkleNodeType::Leaf,
            })
            .collect();

        // Build tree bottom-up
        // Note: We continue until we have exactly one node (the root)
        while nodes.len() > 1 {
            let mut next_level: Vec<MerkleNode> = Vec::new();

            for chunk in nodes.chunks(2) {
                if chunk.len() == 2 {
                    // Pair two nodes
                    let left = chunk[0].clone();
                    let right = chunk[1].clone();
                    let combined_hash = Self::hash_pair(&left.hash, &right.hash);

                    next_level.push(MerkleNode {
                        hash: combined_hash,
                        left: Some(Box::new(left)),
                        right: Some(Box::new(right)),
                        data_id: None,
                        node_type: MerkleNodeType::Internal,
                    });
                } else {
                    // Odd node - pair with empty node
                    let left = chunk[0].clone();
                    let empty_hash = Self::empty_hash();
                    let combined_hash = Self::hash_pair(&left.hash, &empty_hash);

                    let empty_node = MerkleNode {
                        hash: empty_hash,
                        left: None,
                        right: None,
                        data_id: None,
                        node_type: MerkleNodeType::Empty,
                    };

                    next_level.push(MerkleNode {
                        hash: combined_hash,
                        left: Some(Box::new(left)),
                        right: Some(Box::new(empty_node)),
                        data_id: None,
                        node_type: MerkleNodeType::Internal,
                    });
                }
            }

            nodes = next_level;
        }

        // Handle single leaf case: wrap it in a parent node with empty sibling
        let root = if nodes.len() == 1 && nodes[0].node_type == MerkleNodeType::Leaf {
            let leaf = nodes.into_iter().next().unwrap();
            let empty_hash = Self::empty_hash();
            let combined_hash = Self::hash_pair(&leaf.hash, &empty_hash);
            let empty_node = MerkleNode {
                hash: empty_hash,
                left: None,
                right: None,
                data_id: None,
                node_type: MerkleNodeType::Empty,
            };
            Some(MerkleNode {
                hash: combined_hash,
                left: Some(Box::new(leaf)),
                right: Some(Box::new(empty_node)),
                data_id: None,
                node_type: MerkleNodeType::Internal,
            })
        } else {
            nodes.into_iter().next()
        };

        MerkleTree {
            root,
            leaves,
            leaf_ids,
        }
    }

    /// Get the root hash of the tree
    pub fn root(&self) -> Option<&str> {
        self.root.as_ref().map(|n| n.hash.as_str())
    }

    /// Get the root node of the tree
    pub fn root_node(&self) -> Option<&MerkleNode> {
        self.root.as_ref()
    }

    /// Get the height of the tree
    pub fn height(&self) -> usize {
        if self.leaves.is_empty() {
            return 0;
        }
        // Height = ceil(log2(n)) + 1 for n leaves
        let n = self.leaves.len();
        ((n as f64).log2().ceil() as usize) + 1
    }

    /// Get the number of leaves in the tree
    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    /// Get the leaves (hashes) in order
    pub fn leaves(&self) -> &[String] {
        &self.leaves
    }

    /// Generate a proof that a leaf at the given index exists in the tree
    pub fn generate_proof(&self, leaf_index: usize) -> Result<MerkleProof, MerkleError> {
        if self.leaves.is_empty() {
            return Err(MerkleError::EmptyTree);
        }

        if leaf_index >= self.leaves.len() {
            return Err(MerkleError::LeafIndexOutOfBounds {
                index: leaf_index,
                max: self.leaves.len() - 1,
            });
        }

        let leaf_hash = self.leaves[leaf_index].clone();
        let root_hash = self.root().ok_or(MerkleError::EmptyTree)?.to_string();

        // Build proof by traversing from leaf to root
        let siblings = self.collect_proof_siblings(leaf_index);

        Ok(MerkleProof {
            leaf_hash,
            leaf_index,
            siblings,
            root: root_hash,
        })
    }

    /// Generate a proof by leaf hash (finds the leaf first)
    pub fn generate_proof_by_hash(&self, leaf_hash: &str) -> Result<MerkleProof, MerkleError> {
        let index = self
            .leaves
            .iter()
            .position(|h| h == leaf_hash)
            .ok_or_else(|| MerkleError::Other(format!("Leaf hash not found: {}", leaf_hash)))?;

        self.generate_proof(index)
    }

    /// Verify a Merkle proof against an expected root
    pub fn verify_proof(proof: &MerkleProof, expected_root: &str) -> bool {
        let mut current_hash = proof.leaf_hash.clone();

        for sibling in &proof.siblings {
            current_hash = match sibling.position {
                SiblingPosition::Left => Self::hash_pair(&sibling.hash, &current_hash),
                SiblingPosition::Right => Self::hash_pair(&current_hash, &sibling.hash),
            };
        }

        current_hash == expected_root
    }

    /// Verify a proof against this tree's root
    pub fn verify_proof_internal(&self, proof: &MerkleProof) -> bool {
        match self.root() {
            Some(root) => Self::verify_proof(proof, root),
            None => false,
        }
    }

    /// Hash two values together using BLAKE3
    pub fn hash_pair(left: &str, right: &str) -> String {
        let combined = format!("{}|{}", left, right);
        blake3::hash(combined.as_bytes()).to_hex().to_string()
    }

    /// Generate a hash for a single value using BLAKE3
    pub fn hash_single(data: &str) -> String {
        blake3::hash(data.as_bytes()).to_hex().to_string()
    }

    /// Generate an empty hash (for padding unbalanced trees)
    pub fn empty_hash() -> String {
        blake3::hash(b"EMPTY_NODE").to_hex().to_string()
    }

    /// Collect proof siblings using flattened index computation
    ///
    /// This uses a simpler algorithm that computes sibling hashes level by level.
    fn collect_proof_siblings(&self, leaf_index: usize) -> Vec<ProofSibling> {
        let n = self.leaves.len();
        if n == 0 {
            return Vec::new();
        }

        // Build all level hashes
        let levels = self.build_level_hashes();

        let mut siblings = Vec::new();
        let mut idx = leaf_index;

        // Traverse from leaf level (level 0) up to root
        for level in &levels[..levels.len().saturating_sub(1)] {
            let sibling_idx = idx ^ 1; // XOR to get sibling index

            if sibling_idx < level.len() {
                let sibling_hash = level[sibling_idx].clone();
                let position = if idx % 2 == 0 {
                    SiblingPosition::Right // sibling is on the right
                } else {
                    SiblingPosition::Left // sibling is on the left
                };

                siblings.push(ProofSibling {
                    hash: sibling_hash,
                    position,
                });
            } else {
                // Sibling doesn't exist (unbalanced tree), use empty hash
                siblings.push(ProofSibling {
                    hash: Self::empty_hash(),
                    position: SiblingPosition::Right,
                });
            }

            idx /= 2; // Move to parent index
        }

        siblings
    }

    /// Build hashes for each level of the tree
    /// Returns Vec<Vec<String>> where levels[0] is leaves, levels[n-1] is root
    fn build_level_hashes(&self) -> Vec<Vec<String>> {
        let mut levels: Vec<Vec<String>> = Vec::new();

        // Level 0: leaves (already sorted)
        levels.push(self.leaves.clone());

        // Build each subsequent level
        let mut current_level = self.leaves.clone();
        let empty = Self::empty_hash();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..current_level.len()).step_by(2) {
                let left = &current_level[i];
                let right = if i + 1 < current_level.len() {
                    &current_level[i + 1]
                } else {
                    // Pad with empty hash for odd number of nodes
                    &empty
                };
                next_level.push(Self::hash_pair(left, right));
            }

            levels.push(next_level.clone());
            current_level = next_level;
        }

        levels
    }

    /// Create a summary of this tree's root
    pub fn create_summary(
        &self,
        entity_type: MerkleEntityType,
        entity_id: String,
    ) -> Option<MerkleRootSummary> {
        self.root().map(|root_hash| MerkleRootSummary {
            root_hash: root_hash.to_string(),
            leaf_count: self.leaf_count(),
            tree_height: self.height(),
            computed_at: Utc::now(),
            entity_type,
            entity_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let tree = MerkleTree::new();
        assert!(tree.root().is_none());
        assert_eq!(tree.height(), 0);
        assert_eq!(tree.leaf_count(), 0);
    }

    #[test]
    fn test_single_leaf() {
        let leaves = vec!["hash1".to_string()];
        let tree = MerkleTree::from_leaves(leaves);

        assert!(tree.root().is_some());
        assert_eq!(tree.leaf_count(), 1);
        assert_eq!(tree.height(), 1);

        // For single leaf, root should be the leaf hash itself
        // (paired with empty hash)
        let expected_root = MerkleTree::hash_pair("hash1", &MerkleTree::empty_hash());
        assert_eq!(tree.root().unwrap(), expected_root);
    }

    #[test]
    fn test_two_leaves() {
        let leaves = vec!["hash1".to_string(), "hash2".to_string()];
        let tree = MerkleTree::from_leaves(leaves);

        assert!(tree.root().is_some());
        assert_eq!(tree.leaf_count(), 2);
        assert_eq!(tree.height(), 2);

        // Leaves are sorted, so order may differ
        let sorted_hashes = {
            let mut h = vec!["hash1", "hash2"];
            h.sort();
            h
        };
        let expected_root = MerkleTree::hash_pair(sorted_hashes[0], sorted_hashes[1]);
        assert_eq!(tree.root().unwrap(), expected_root);
    }

    #[test]
    fn test_three_leaves() {
        let leaves = vec![
            "hash1".to_string(),
            "hash2".to_string(),
            "hash3".to_string(),
        ];
        let tree = MerkleTree::from_leaves(leaves);

        assert!(tree.root().is_some());
        assert_eq!(tree.leaf_count(), 3);
        // Height for 3 leaves: ceil(log2(3)) + 1 = 2 + 1 = 3
        assert_eq!(tree.height(), 3);
    }

    #[test]
    fn test_four_leaves() {
        let leaves = vec![
            "hash1".to_string(),
            "hash2".to_string(),
            "hash3".to_string(),
            "hash4".to_string(),
        ];
        let tree = MerkleTree::from_leaves(leaves);

        assert!(tree.root().is_some());
        assert_eq!(tree.leaf_count(), 4);
        assert_eq!(tree.height(), 3);
    }

    #[test]
    fn test_deterministic_ordering() {
        // Same leaves in different order should produce same root
        let leaves1 = vec![
            "hash3".to_string(),
            "hash1".to_string(),
            "hash2".to_string(),
        ];
        let leaves2 = vec![
            "hash1".to_string(),
            "hash2".to_string(),
            "hash3".to_string(),
        ];

        let tree1 = MerkleTree::from_leaves(leaves1);
        let tree2 = MerkleTree::from_leaves(leaves2);

        assert_eq!(tree1.root(), tree2.root());
    }

    #[test]
    fn test_proof_generation_and_verification() {
        let leaves = vec![
            "hash1".to_string(),
            "hash2".to_string(),
            "hash3".to_string(),
            "hash4".to_string(),
        ];
        let tree = MerkleTree::from_leaves(leaves);

        // Generate proof for first leaf (after sorting)
        let proof = tree.generate_proof(0).unwrap();

        // Verify the proof
        assert!(MerkleTree::verify_proof(&proof, tree.root().unwrap()));

        // Verify internal method works too
        assert!(tree.verify_proof_internal(&proof));
    }

    #[test]
    fn test_proof_for_each_leaf() {
        let leaves = vec![
            "event1".to_string(),
            "event2".to_string(),
            "event3".to_string(),
            "event4".to_string(),
            "event5".to_string(),
        ];
        let tree = MerkleTree::from_leaves(leaves);

        // Verify proof works for each leaf
        for i in 0..tree.leaf_count() {
            let proof = tree.generate_proof(i).unwrap();
            assert!(
                MerkleTree::verify_proof(&proof, tree.root().unwrap()),
                "Proof failed for leaf index {}",
                i
            );
        }
    }

    #[test]
    fn test_proof_by_hash() {
        let leaves = vec![
            "hash1".to_string(),
            "hash2".to_string(),
            "hash3".to_string(),
        ];
        let tree = MerkleTree::from_leaves(leaves);

        // Find and verify proof by hash
        let proof = tree.generate_proof_by_hash("hash2").unwrap();
        assert!(MerkleTree::verify_proof(&proof, tree.root().unwrap()));
    }

    #[test]
    fn test_invalid_proof_fails() {
        let leaves = vec![
            "hash1".to_string(),
            "hash2".to_string(),
            "hash3".to_string(),
        ];
        let tree = MerkleTree::from_leaves(leaves);

        let proof = tree.generate_proof(0).unwrap();

        // Verify against wrong root should fail
        assert!(!MerkleTree::verify_proof(&proof, "wrong_root"));
    }

    #[test]
    fn test_leaf_index_out_of_bounds() {
        let leaves = vec!["hash1".to_string(), "hash2".to_string()];
        let tree = MerkleTree::from_leaves(leaves);

        let result = tree.generate_proof(10);
        assert!(matches!(
            result,
            Err(MerkleError::LeafIndexOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_proof_on_empty_tree() {
        let tree = MerkleTree::new();
        let result = tree.generate_proof(0);
        assert!(matches!(result, Err(MerkleError::EmptyTree)));
    }

    #[test]
    fn test_create_summary() {
        let leaves = vec![
            "hash1".to_string(),
            "hash2".to_string(),
            "hash3".to_string(),
        ];
        let tree = MerkleTree::from_leaves(leaves);

        let summary = tree
            .create_summary(MerkleEntityType::Item, "DFID-123".to_string())
            .unwrap();

        assert_eq!(summary.root_hash, tree.root().unwrap());
        assert_eq!(summary.leaf_count, 3);
        assert_eq!(summary.entity_type, MerkleEntityType::Item);
        assert_eq!(summary.entity_id, "DFID-123");
    }

    #[test]
    fn test_hash_consistency() {
        // Same input should always produce same hash
        let hash1 = MerkleTree::hash_single("test_data");
        let hash2 = MerkleTree::hash_single("test_data");
        assert_eq!(hash1, hash2);

        // Different input should produce different hash
        let hash3 = MerkleTree::hash_single("different_data");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_pair_order_matters() {
        let hash1 = MerkleTree::hash_pair("a", "b");
        let hash2 = MerkleTree::hash_pair("b", "a");
        assert_ne!(hash1, hash2);
    }
}
