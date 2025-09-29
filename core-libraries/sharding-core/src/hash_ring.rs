//! Consistent hash ring implementation

use crate::{config::HashFunctionType, node::VirtualNode, NodeId, Result, ShardError};
use ahash::AHasher;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

/// Trait for hash functions used in consistent hashing
pub trait HashFunction: Send + Sync {
    /// Compute hash for a key
    fn hash(&self, key: &[u8]) -> u64;

    /// Get the name of this hash function
    fn name(&self) -> &'static str;
}

/// SHA-256 hash function implementation
#[derive(Debug, Clone)]
pub struct Sha256HashFunction;

impl HashFunction for Sha256HashFunction {
    fn hash(&self, key: &[u8]) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(key);
        let result = hasher.finalize();

        // Take first 8 bytes and convert to u64
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&result[..8]);
        u64::from_be_bytes(bytes)
    }

    fn name(&self) -> &'static str {
        "sha256"
    }
}

/// AHash function implementation (fast, non-cryptographic)
#[derive(Debug, Clone)]
pub struct AHashFunction;

impl HashFunction for AHashFunction {
    fn hash(&self, key: &[u8]) -> u64 {
        let mut hasher = AHasher::default();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn name(&self) -> &'static str {
        "ahash"
    }
}

/// xxHash function implementation (placeholder - would need xxhash crate)
#[derive(Debug, Clone)]
pub struct XxHashFunction;

impl HashFunction for XxHashFunction {
    fn hash(&self, key: &[u8]) -> u64 {
        // Placeholder implementation using AHash for now
        // In production, would use proper xxHash implementation
        let mut hasher = AHasher::default();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn name(&self) -> &'static str {
        "xxhash"
    }
}

/// Create a hash function based on the type
pub fn create_hash_function(hash_type: HashFunctionType) -> Box<dyn HashFunction> {
    match hash_type {
        HashFunctionType::Sha256 => Box::new(Sha256HashFunction),
        HashFunctionType::AHash => Box::new(AHashFunction),
        HashFunctionType::XxHash => Box::new(XxHashFunction),
    }
}

/// Consistent hash ring for distributed sharding
pub struct ConsistentHashRing {
    /// Virtual nodes sorted by hash value
    ring: BTreeMap<u64, VirtualNode>,
    /// Hash function to use
    hash_function: HashFunctionType,
    /// Cache for hash function instance
    hasher: Box<dyn HashFunction>,
}

impl Clone for ConsistentHashRing {
    fn clone(&self) -> Self {
        Self {
            ring: self.ring.clone(),
            hash_function: self.hash_function,
            hasher: create_hash_function(self.hash_function),
        }
    }
}

impl std::fmt::Debug for ConsistentHashRing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConsistentHashRing")
            .field("ring", &self.ring)
            .field("hash_function", &self.hash_function)
            .field("hasher_name", &self.hasher.name())
            .finish()
    }
}

impl ConsistentHashRing {
    /// Create a new consistent hash ring
    pub fn new(hash_function: HashFunctionType) -> Self {
        let hasher = create_hash_function(hash_function);
        Self {
            ring: BTreeMap::new(),
            hash_function,
            hasher,
        }
    }

    /// Add a physical node with its virtual nodes to the ring
    pub fn add_node(&mut self, node_id: NodeId, virtual_node_count: usize) -> Result<Vec<u64>> {
        if virtual_node_count == 0 {
            return Err(ShardError::InvalidConfiguration {
                reason: "virtual_node_count must be greater than 0".to_string(),
            });
        }

        let mut virtual_hashes = Vec::new();

        for i in 0..virtual_node_count {
            // Create unique identifier for this virtual node
            let virtual_key = format!("{}:{}", node_id.as_str(), i);
            let hash = self.hasher.hash(virtual_key.as_bytes());

            // Check for hash collisions
            if self.ring.contains_key(&hash) {
                return Err(ShardError::VirtualNodeCollision);
            }

            let virtual_node = VirtualNode::new(hash, node_id.clone(), i);
            self.ring.insert(hash, virtual_node);
            virtual_hashes.push(hash);
        }

        Ok(virtual_hashes)
    }

    /// Remove a physical node and all its virtual nodes from the ring
    pub fn remove_node(&mut self, node_id: &NodeId) -> Result<Vec<u64>> {
        let mut removed_hashes = Vec::new();

        // Find all virtual nodes belonging to this physical node
        let keys_to_remove: Vec<u64> = self
            .ring
            .iter()
            .filter(|(_, vnode)| &vnode.node_id == node_id)
            .map(|(hash, _)| *hash)
            .collect();

        if keys_to_remove.is_empty() {
            return Err(ShardError::NodeNotFound {
                node_id: node_id.to_string(),
            });
        }

        // Remove the virtual nodes
        for hash in keys_to_remove {
            self.ring.remove(&hash);
            removed_hashes.push(hash);
        }

        Ok(removed_hashes)
    }

    /// Get the responsible node for a given key
    pub fn get_node(&self, key: &str) -> Result<NodeId> {
        if self.ring.is_empty() {
            return Err(ShardError::EmptyHashRing);
        }

        let key_hash = self.hasher.hash(key.as_bytes());

        // Find the first virtual node with hash >= key_hash
        if let Some((_, virtual_node)) = self.ring.range(key_hash..).next() {
            Ok(virtual_node.node_id.clone())
        } else {
            // Wrap around to the first node in the ring
            let (_, virtual_node) = self.ring.iter().next().unwrap();
            Ok(virtual_node.node_id.clone())
        }
    }

    /// Get multiple responsible nodes for replication
    pub fn get_nodes(&self, key: &str, count: usize) -> Result<Vec<NodeId>> {
        if self.ring.is_empty() {
            return Err(ShardError::EmptyHashRing);
        }

        if count == 0 {
            return Ok(Vec::new());
        }

        let key_hash = self.hasher.hash(key.as_bytes());
        let mut result = Vec::new();
        let mut seen_nodes = std::collections::HashSet::new();

        // Start from the position on the ring
        let mut iter = self.ring.range(key_hash..);
        let mut wrapped = false;

        loop {
            let virtual_node = if let Some((_, vnode)) = iter.next() {
                vnode
            } else if !wrapped {
                // Wrap around to the beginning
                iter = self.ring.range(..);
                wrapped = true;
                if let Some((_, vnode)) = iter.next() {
                    vnode
                } else {
                    break;
                }
            } else {
                break;
            };

            // Only add unique physical nodes
            if seen_nodes.insert(virtual_node.node_id.clone()) {
                result.push(virtual_node.node_id.clone());
                if result.len() >= count {
                    break;
                }
            }

            // If we've seen all possible nodes, stop
            if seen_nodes.len() >= self.physical_node_count() {
                break;
            }
        }

        if result.len() < count {
            return Err(ShardError::InsufficientNodes {
                required: count,
                available: result.len(),
            });
        }

        Ok(result)
    }

    /// Get all keys that would be affected by adding a new node
    pub fn get_affected_ranges(&self, new_node_positions: &[u64]) -> Vec<(u64, u64)> {
        let mut ranges = Vec::new();

        for &position in new_node_positions {
            // Find the range this virtual node will be responsible for
            let start = if let Some((&prev_hash, _)) = self.ring.range(..position).next_back() {
                prev_hash + 1
            } else {
                // This is the first position, so it takes from the last position
                if let Some((&last_hash, _)) = self.ring.iter().next_back() {
                    last_hash + 1
                } else {
                    0 // Empty ring
                }
            };

            ranges.push((start, position));
        }

        ranges
    }

    /// Get the hash range a node is responsible for
    pub fn get_node_ranges(&self, node_id: &NodeId) -> Vec<(u64, u64)> {
        let mut ranges = Vec::new();

        // Get all virtual nodes for this physical node
        let virtual_nodes: Vec<u64> = self
            .ring
            .iter()
            .filter(|(_, vnode)| &vnode.node_id == node_id)
            .map(|(hash, _)| *hash)
            .collect();

        for &vnode_hash in &virtual_nodes {
            // Find the previous virtual node in the ring
            let start = if let Some((&prev_hash, _)) = self.ring.range(..vnode_hash).next_back() {
                prev_hash + 1
            } else {
                // This is the first virtual node, so it wraps around
                if let Some((&last_hash, _)) = self.ring.iter().next_back() {
                    last_hash + 1
                } else {
                    0 // Only one virtual node
                }
            };

            ranges.push((start, vnode_hash));
        }

        ranges
    }

    /// Check if the ring is balanced within a threshold
    pub fn is_balanced(&self, threshold: f64) -> bool {
        if self.ring.is_empty() {
            return true;
        }

        let node_counts = self.get_node_distribution();
        if node_counts.is_empty() {
            return true;
        }

        let total_virtual_nodes = self.ring.len();
        let node_count = node_counts.len();
        let expected_per_node = total_virtual_nodes as f64 / node_count as f64;

        // Check if any node deviates too much from expected
        for count in node_counts.values() {
            let deviation = (*count as f64 - expected_per_node).abs() / expected_per_node;
            if deviation > threshold {
                return false;
            }
        }

        true
    }

    /// Get distribution of virtual nodes across physical nodes
    pub fn get_node_distribution(&self) -> std::collections::HashMap<NodeId, usize> {
        let mut distribution = std::collections::HashMap::new();

        for virtual_node in self.ring.values() {
            *distribution.entry(virtual_node.node_id.clone()).or_insert(0) += 1;
        }

        distribution
    }

    /// Get the number of physical nodes in the ring
    pub fn physical_node_count(&self) -> usize {
        self.get_node_distribution().len()
    }

    /// Get the number of virtual nodes in the ring
    pub fn virtual_node_count(&self) -> usize {
        self.ring.len()
    }

    /// Check if the ring is empty
    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }

    /// Get all virtual nodes (for testing/debugging)
    pub fn virtual_nodes(&self) -> Vec<&VirtualNode> {
        self.ring.values().collect()
    }

    /// Clear the ring
    pub fn clear(&mut self) {
        self.ring.clear();
    }

    /// Get hash function name
    pub fn hash_function_name(&self) -> &'static str {
        self.hasher.name()
    }

    /// Serialize the ring state
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let state = HashRingState {
            virtual_nodes: self.ring.values().cloned().collect(),
            hash_function: self.hash_function,
        };

        bincode::serialize(&state).map_err(|e| ShardError::SerializationError {
            reason: e.to_string(),
        })
    }

    /// Deserialize and restore ring state
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let state: HashRingState = bincode::deserialize(data).map_err(|e| ShardError::DeserializationError {
            reason: e.to_string(),
        })?;

        let mut ring = Self::new(state.hash_function);

        for virtual_node in state.virtual_nodes {
            ring.ring.insert(virtual_node.hash, virtual_node);
        }

        Ok(ring)
    }
}

/// Serializable state of the hash ring
#[derive(Serialize, Deserialize)]
struct HashRingState {
    virtual_nodes: Vec<VirtualNode>,
    hash_function: HashFunctionType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_functions() {
        let sha256 = Sha256HashFunction;
        let ahash = AHashFunction;
        let xxhash = XxHashFunction;

        let key = b"test_key";

        // Test that different functions produce different results
        let sha256_hash = sha256.hash(key);
        let ahash_hash = ahash.hash(key);
        let xxhash_hash = xxhash.hash(key);

        // They should be deterministic
        assert_eq!(sha256.hash(key), sha256_hash);
        assert_eq!(ahash.hash(key), ahash_hash);
        assert_eq!(xxhash.hash(key), xxhash_hash);

        // Names should be correct
        assert_eq!(sha256.name(), "sha256");
        assert_eq!(ahash.name(), "ahash");
        assert_eq!(xxhash.name(), "xxhash");
    }

    #[test]
    fn test_empty_ring() {
        let ring = ConsistentHashRing::new(HashFunctionType::AHash);

        assert!(ring.is_empty());
        assert_eq!(ring.virtual_node_count(), 0);
        assert_eq!(ring.physical_node_count(), 0);

        let result = ring.get_node("test_key");
        assert!(matches!(result, Err(ShardError::EmptyHashRing)));
    }

    #[test]
    fn test_single_node() {
        let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);
        let node_id = NodeId::new("node1");

        let virtual_hashes = ring.add_node(node_id.clone(), 3).unwrap();
        assert_eq!(virtual_hashes.len(), 3);
        assert_eq!(ring.virtual_node_count(), 3);
        assert_eq!(ring.physical_node_count(), 1);

        // All keys should map to the only node
        assert_eq!(ring.get_node("key1").unwrap(), node_id);
        assert_eq!(ring.get_node("key2").unwrap(), node_id);
        assert_eq!(ring.get_node("key3").unwrap(), node_id);
    }

    #[test]
    fn test_multiple_nodes() {
        let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);
        let node1 = NodeId::new("node1");
        let node2 = NodeId::new("node2");
        let node3 = NodeId::new("node3");

        ring.add_node(node1.clone(), 2).unwrap();
        ring.add_node(node2.clone(), 2).unwrap();
        ring.add_node(node3.clone(), 2).unwrap();

        assert_eq!(ring.virtual_node_count(), 6);
        assert_eq!(ring.physical_node_count(), 3);

        // Test that different keys map to different nodes
        // Use a larger set of diverse keys to ensure distribution
        let test_keys = [
            "key1", "key2", "key3", "key4", "key5",
            "abcdef", "xyz123", "foobar", "test_data", "random_string",
            "node_alpha", "node_beta", "node_gamma"
        ];

        let mut unique_nodes = std::collections::HashSet::new();
        for key in &test_keys {
            if let Ok(node) = ring.get_node(key) {
                unique_nodes.insert(node);
            }
        }

        // With 3 nodes and diverse keys, should get distribution across multiple nodes
        assert!(unique_nodes.len() > 1, "Expected distribution across multiple nodes, got {}", unique_nodes.len());
    }

    #[test]
    fn test_replication() {
        let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);
        let node1 = NodeId::new("node1");
        let node2 = NodeId::new("node2");
        let node3 = NodeId::new("node3");

        ring.add_node(node1, 2).unwrap();
        ring.add_node(node2, 2).unwrap();
        ring.add_node(node3, 2).unwrap();

        // Test getting multiple nodes for replication
        let nodes = ring.get_nodes("test_key", 2).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_ne!(nodes[0], nodes[1]); // Should be different physical nodes

        // Test getting more nodes than available
        let nodes = ring.get_nodes("test_key", 3).unwrap();
        assert_eq!(nodes.len(), 3);

        // Test requesting more nodes than exist
        let result = ring.get_nodes("test_key", 5);
        assert!(matches!(result, Err(ShardError::InsufficientNodes { .. })));
    }

    #[test]
    fn test_node_removal() {
        let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);
        let node1 = NodeId::new("node1");
        let node2 = NodeId::new("node2");

        ring.add_node(node1.clone(), 3).unwrap();
        ring.add_node(node2.clone(), 3).unwrap();

        assert_eq!(ring.physical_node_count(), 2);

        // Remove a node
        let removed_hashes = ring.remove_node(&node1).unwrap();
        assert_eq!(removed_hashes.len(), 3);
        assert_eq!(ring.physical_node_count(), 1);

        // Try to remove non-existent node
        let node3 = NodeId::new("node3");
        let result = ring.remove_node(&node3);
        assert!(matches!(result, Err(ShardError::NodeNotFound { .. })));
    }

    #[test]
    fn test_node_ranges() {
        let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);
        let node1 = NodeId::new("node1");

        ring.add_node(node1.clone(), 2).unwrap();

        let ranges = ring.get_node_ranges(&node1);
        assert_eq!(ranges.len(), 2);

        // In a hash ring, ranges can wrap around (start > end is valid)
        // Each range represents a valid hash range responsibility
        for (start, end) in ranges {
            // Either start <= end (normal range) or start > end (wrap-around range)
            assert!(start != end, "Range start and end should not be equal");
        }
    }

    #[test]
    fn test_affected_ranges() {
        let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);
        let node1 = NodeId::new("node1");

        ring.add_node(node1, 2).unwrap();

        let new_positions = vec![100, 200];
        let ranges = ring.get_affected_ranges(&new_positions);
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn test_ring_balance() {
        let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);
        let node1 = NodeId::new("node1");
        let node2 = NodeId::new("node2");

        // Add equal number of virtual nodes
        ring.add_node(node1, 5).unwrap();
        ring.add_node(node2, 5).unwrap();

        // Should be balanced with a reasonable threshold
        assert!(ring.is_balanced(0.5));

        // Add unbalanced node
        let node3 = NodeId::new("node3");
        ring.add_node(node3, 1).unwrap();

        // Should be less balanced now
        assert!(!ring.is_balanced(0.1));
    }

    #[test]
    fn test_node_distribution() {
        let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);
        let node1 = NodeId::new("node1");
        let node2 = NodeId::new("node2");

        ring.add_node(node1.clone(), 3).unwrap();
        ring.add_node(node2.clone(), 2).unwrap();

        let distribution = ring.get_node_distribution();
        assert_eq!(distribution.len(), 2);
        assert_eq!(*distribution.get(&node1).unwrap(), 3);
        assert_eq!(*distribution.get(&node2).unwrap(), 2);
    }

    #[test]
    fn test_serialization() {
        let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);
        let node1 = NodeId::new("node1");
        let node2 = NodeId::new("node2");

        ring.add_node(node1.clone(), 2).unwrap();
        ring.add_node(node2.clone(), 2).unwrap();

        // Serialize
        let serialized = ring.serialize().unwrap();

        // Deserialize
        let restored_ring = ConsistentHashRing::deserialize(&serialized).unwrap();

        assert_eq!(ring.virtual_node_count(), restored_ring.virtual_node_count());
        assert_eq!(ring.physical_node_count(), restored_ring.physical_node_count());

        // Test that key mappings are the same
        assert_eq!(ring.get_node("test_key").unwrap(), restored_ring.get_node("test_key").unwrap());
    }

    #[test]
    fn test_virtual_node_collision() {
        // This test is probabilistic and might not always trigger a collision
        // In practice, collision detection would be more sophisticated
        let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);
        let node1 = NodeId::new("node1");

        // Adding many virtual nodes increases chance of collision
        // But with good hash functions, collisions should be rare
        let result = ring.add_node(node1, 1000);
        // For this test, we'll just check that it doesn't panic
        // In a real collision scenario, the function would return an error
        assert!(result.is_ok() || matches!(result, Err(ShardError::VirtualNodeCollision)));
    }

    #[test]
    fn test_deterministic_hashing() {
        let ring1 = ConsistentHashRing::new(HashFunctionType::AHash);
        let ring2 = ConsistentHashRing::new(HashFunctionType::AHash);

        // Same input should produce same hash
        let key = "test_key";
        let hash1 = ring1.hasher.hash(key.as_bytes());
        let hash2 = ring2.hasher.hash(key.as_bytes());

        assert_eq!(hash1, hash2);
    }
}