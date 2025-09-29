//! Message codec implementations for serialization/deserialization

use crate::{NetworkError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Message codec trait for encoding/decoding messages
#[async_trait]
pub trait MessageCodec<T>: Send + Sync {
    /// Encode a message to bytes
    async fn encode(&self, message: &T) -> Result<Vec<u8>>;

    /// Decode bytes to a message
    async fn decode(&self, data: &[u8]) -> Result<T>;

    /// Get the codec name for diagnostics
    fn name(&self) -> &'static str;

    /// Get maximum supported message size
    fn max_size(&self) -> Option<usize> {
        None
    }
}

/// Binary codec using bincode for efficient serialization
pub struct BinaryCodec<T> {
    max_size: Option<usize>,
    _phantom: PhantomData<T>,
}

impl<T> BinaryCodec<T> {
    /// Create a new binary codec
    pub fn new() -> Self {
        Self {
            max_size: None,
            _phantom: PhantomData,
        }
    }

    /// Create a binary codec with maximum message size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            max_size: Some(max_size),
            _phantom: PhantomData,
        }
    }
}

impl<T> Default for BinaryCodec<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<T> MessageCodec<T> for BinaryCodec<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync,
{
    async fn encode(&self, message: &T) -> Result<Vec<u8>> {
        let data = bincode::serialize(message).map_err(|e| NetworkError::SerializationError {
            reason: format!("bincode error: {}", e),
        })?;

        // Check size limit
        if let Some(max_size) = self.max_size {
            if data.len() > max_size {
                return Err(NetworkError::MessageTooLarge {
                    size: data.len(),
                    max_size,
                });
            }
        }

        Ok(data)
    }

    async fn decode(&self, data: &[u8]) -> Result<T> {
        // Check size limit
        if let Some(max_size) = self.max_size {
            if data.len() > max_size {
                return Err(NetworkError::MessageTooLarge {
                    size: data.len(),
                    max_size,
                });
            }
        }

        bincode::deserialize(data).map_err(|e| NetworkError::DeserializationError {
            reason: format!("bincode error: {}", e),
        })
    }

    fn name(&self) -> &'static str {
        "binary"
    }

    fn max_size(&self) -> Option<usize> {
        self.max_size
    }
}

/// JSON codec for human-readable serialization
pub struct JsonCodec<T> {
    max_size: Option<usize>,
    pretty: bool,
    _phantom: PhantomData<T>,
}

impl<T> JsonCodec<T> {
    /// Create a new JSON codec
    pub fn new() -> Self {
        Self {
            max_size: None,
            pretty: false,
            _phantom: PhantomData,
        }
    }

    /// Create a JSON codec with maximum message size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            max_size: Some(max_size),
            pretty: false,
            _phantom: PhantomData,
        }
    }

    /// Create a pretty-printed JSON codec
    pub fn pretty() -> Self {
        Self {
            max_size: None,
            pretty: true,
            _phantom: PhantomData,
        }
    }

    /// Create a pretty-printed JSON codec with size limit
    pub fn pretty_with_max_size(max_size: usize) -> Self {
        Self {
            max_size: Some(max_size),
            pretty: true,
            _phantom: PhantomData,
        }
    }
}

impl<T> Default for JsonCodec<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<T> MessageCodec<T> for JsonCodec<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync,
{
    async fn encode(&self, message: &T) -> Result<Vec<u8>> {
        let data = if self.pretty {
            serde_json::to_vec_pretty(message)
        } else {
            serde_json::to_vec(message)
        }.map_err(|e| NetworkError::SerializationError {
            reason: format!("json error: {}", e),
        })?;

        // Check size limit
        if let Some(max_size) = self.max_size {
            if data.len() > max_size {
                return Err(NetworkError::MessageTooLarge {
                    size: data.len(),
                    max_size,
                });
            }
        }

        Ok(data)
    }

    async fn decode(&self, data: &[u8]) -> Result<T> {
        // Check size limit
        if let Some(max_size) = self.max_size {
            if data.len() > max_size {
                return Err(NetworkError::MessageTooLarge {
                    size: data.len(),
                    max_size,
                });
            }
        }

        serde_json::from_slice(data).map_err(|e| NetworkError::DeserializationError {
            reason: format!("json error: {}", e),
        })
    }

    fn name(&self) -> &'static str {
        "json"
    }

    fn max_size(&self) -> Option<usize> {
        self.max_size
    }
}

/// Compressed codec wrapper that adds compression to any codec
pub struct CompressedCodec<T, C> {
    inner_codec: C,
    compression_level: u8,
    min_size: usize,
    _phantom: PhantomData<T>,
}

impl<T, C> CompressedCodec<T, C>
where
    C: MessageCodec<T>,
{
    /// Create a new compressed codec
    pub fn new(inner_codec: C) -> Self {
        Self {
            inner_codec,
            compression_level: 6, // Default compression level
            min_size: 1024, // Only compress messages larger than 1KB
            _phantom: PhantomData,
        }
    }

    /// Create compressed codec with specific compression level (1-9)
    pub fn with_level(inner_codec: C, level: u8) -> Self {
        Self {
            inner_codec,
            compression_level: level.clamp(1, 9),
            min_size: 1024,
            _phantom: PhantomData,
        }
    }

    /// Set minimum size threshold for compression
    pub fn with_min_size(mut self, min_size: usize) -> Self {
        self.min_size = min_size;
        self
    }

    /// Compress data using gzip
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use std::io::Write;

        if data.len() < self.min_size {
            // Don't compress small messages
            let mut result = Vec::with_capacity(data.len() + 1);
            result.push(0); // Uncompressed marker
            result.extend_from_slice(data);
            return Ok(result);
        }

        let mut encoder = flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::new(self.compression_level as u32),
        );

        encoder.write_all(data).map_err(|e| NetworkError::SerializationError {
            reason: format!("compression error: {}", e),
        })?;

        let compressed = encoder.finish().map_err(|e| NetworkError::SerializationError {
            reason: format!("compression error: {}", e),
        })?;

        // Add compressed marker
        let mut result = Vec::with_capacity(compressed.len() + 1);
        result.push(1); // Compressed marker
        result.extend_from_slice(&compressed);
        Ok(result)
    }

    /// Decompress data using gzip
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Err(NetworkError::DeserializationError {
                reason: "empty compressed data".to_string(),
            });
        }

        let compressed_flag = data[0];
        let payload = &data[1..];

        if compressed_flag == 0 {
            // Uncompressed data
            Ok(payload.to_vec())
        } else if compressed_flag == 1 {
            // Compressed data
            use std::io::Read;

            let mut decoder = flate2::read::GzDecoder::new(payload);
            let mut decompressed = Vec::new();

            decoder.read_to_end(&mut decompressed).map_err(|e| NetworkError::DeserializationError {
                reason: format!("decompression error: {}", e),
            })?;

            Ok(decompressed)
        } else {
            Err(NetworkError::DeserializationError {
                reason: format!("invalid compression flag: {}", compressed_flag),
            })
        }
    }
}

#[async_trait]
impl<T, C> MessageCodec<T> for CompressedCodec<T, C>
where
    T: Send + Sync,
    C: MessageCodec<T> + Send + Sync,
{
    async fn encode(&self, message: &T) -> Result<Vec<u8>> {
        let encoded = self.inner_codec.encode(message).await?;
        self.compress(&encoded)
    }

    async fn decode(&self, data: &[u8]) -> Result<T> {
        let decompressed = self.decompress(data)?;
        self.inner_codec.decode(&decompressed).await
    }

    fn name(&self) -> &'static str {
        "compressed"
    }

    fn max_size(&self) -> Option<usize> {
        self.inner_codec.max_size()
    }
}

/// Versioned codec that handles protocol version negotiation
pub struct VersionedCodec<T> {
    version: u32,
    codecs: std::collections::HashMap<u32, Box<dyn MessageCodec<T>>>,
    default_version: u32,
}

impl<T> VersionedCodec<T>
where
    T: Send + Sync + 'static,
{
    /// Create a new versioned codec
    pub fn new(default_version: u32) -> Self {
        Self {
            version: default_version,
            codecs: std::collections::HashMap::new(),
            default_version,
        }
    }

    /// Add a codec for a specific version
    pub fn add_version(mut self, version: u32, codec: Box<dyn MessageCodec<T>>) -> Self {
        self.codecs.insert(version, codec);
        self
    }

    /// Set the current protocol version
    pub fn set_version(&mut self, version: u32) {
        self.version = version;
    }

    /// Get supported versions
    pub fn supported_versions(&self) -> Vec<u32> {
        let mut versions: Vec<u32> = self.codecs.keys().cloned().collect();
        versions.sort_unstable();
        versions
    }

    /// Negotiate version with peer
    pub fn negotiate_version(&self, peer_versions: &[u32]) -> Option<u32> {
        // Find highest common version
        let mut common_versions: Vec<u32> = self.codecs.keys()
            .filter(|v| peer_versions.contains(v))
            .cloned()
            .collect();

        common_versions.sort_unstable();
        common_versions.last().cloned()
    }
}

#[async_trait]
impl<T> MessageCodec<T> for VersionedCodec<T>
where
    T: Send + Sync,
{
    async fn encode(&self, message: &T) -> Result<Vec<u8>> {
        let codec = self.codecs.get(&self.version)
            .ok_or_else(|| NetworkError::SerializationError {
                reason: format!("no codec for version {}", self.version),
            })?;

        let payload = codec.encode(message).await?;

        // Prepend version header
        let mut result = Vec::with_capacity(payload.len() + 4);
        result.extend_from_slice(&self.version.to_le_bytes());
        result.extend_from_slice(&payload);

        Ok(result)
    }

    async fn decode(&self, data: &[u8]) -> Result<T> {
        if data.len() < 4 {
            return Err(NetworkError::DeserializationError {
                reason: "message too short for version header".to_string(),
            });
        }

        let version = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let payload = &data[4..];

        let codec = self.codecs.get(&version)
            .ok_or_else(|| NetworkError::DeserializationError {
                reason: format!("unsupported version: {}", version),
            })?;

        codec.decode(payload).await
    }

    fn name(&self) -> &'static str {
        "versioned"
    }

    fn max_size(&self) -> Option<usize> {
        // Return the minimum max_size among all codecs
        self.codecs.values()
            .filter_map(|c| c.max_size())
            .min()
    }
}

/// Create a standard binary codec
pub fn binary_codec<T>() -> BinaryCodec<T> {
    BinaryCodec::new()
}

/// Create a standard JSON codec
pub fn json_codec<T>() -> JsonCodec<T> {
    JsonCodec::new()
}

/// Create a compressed binary codec
pub fn compressed_binary_codec<T>() -> CompressedCodec<T, BinaryCodec<T>>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync,
{
    CompressedCodec::new(BinaryCodec::new())
}

/// Create a compressed JSON codec
pub fn compressed_json_codec<T>() -> CompressedCodec<T, JsonCodec<T>>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync,
{
    CompressedCodec::new(JsonCodec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestMessage {
        id: u64,
        text: String,
        data: Vec<u8>,
    }

    #[tokio::test]
    async fn test_binary_codec() {
        let codec = BinaryCodec::<TestMessage>::new();
        let message = TestMessage {
            id: 123,
            text: "hello".to_string(),
            data: vec![1, 2, 3, 4, 5],
        };

        let encoded = codec.encode(&message).await.unwrap();
        let decoded = codec.decode(&encoded).await.unwrap();

        assert_eq!(message, decoded);
        assert_eq!(codec.name(), "binary");
    }

    #[tokio::test]
    async fn test_json_codec() {
        let codec = JsonCodec::<TestMessage>::new();
        let message = TestMessage {
            id: 456,
            text: "world".to_string(),
            data: vec![6, 7, 8, 9, 10],
        };

        let encoded = codec.encode(&message).await.unwrap();
        let decoded = codec.decode(&encoded).await.unwrap();

        assert_eq!(message, decoded);
        assert_eq!(codec.name(), "json");

        // Test that it's valid JSON
        let json_str = String::from_utf8(encoded).unwrap();
        assert!(json_str.contains("\"id\":456"));
    }

    #[tokio::test]
    async fn test_compressed_codec() {
        let inner_codec = BinaryCodec::<TestMessage>::new();
        let codec = CompressedCodec::new(inner_codec).with_min_size(0); // Compress everything

        let message = TestMessage {
            id: 789,
            text: "compression test with longer text to ensure compression".to_string(),
            data: vec![0; 1000], // Large data to ensure compression
        };

        let encoded = codec.encode(&message).await.unwrap();
        let decoded = codec.decode(&encoded).await.unwrap();

        assert_eq!(message, decoded);
        assert_eq!(codec.name(), "compressed");

        // Verify compression marker
        assert_eq!(encoded[0], 1); // Should be compressed
    }

    #[tokio::test]
    async fn test_versioned_codec() {
        let mut codec = VersionedCodec::<TestMessage>::new(1);
        codec = codec.add_version(1, Box::new(BinaryCodec::new()));
        codec = codec.add_version(2, Box::new(JsonCodec::new()));

        let message = TestMessage {
            id: 999,
            text: "versioned".to_string(),
            data: vec![1, 2, 3],
        };

        // Test with version 1 (binary)
        codec.set_version(1);
        let encoded_v1 = codec.encode(&message).await.unwrap();
        let decoded_v1 = codec.decode(&encoded_v1).await.unwrap();
        assert_eq!(message, decoded_v1);

        // Test with version 2 (JSON)
        codec.set_version(2);
        let encoded_v2 = codec.encode(&message).await.unwrap();
        let decoded_v2 = codec.decode(&encoded_v2).await.unwrap();
        assert_eq!(message, decoded_v2);

        // Verify version headers
        assert_eq!(u32::from_le_bytes([encoded_v1[0], encoded_v1[1], encoded_v1[2], encoded_v1[3]]), 1);
        assert_eq!(u32::from_le_bytes([encoded_v2[0], encoded_v2[1], encoded_v2[2], encoded_v2[3]]), 2);
    }

    #[tokio::test]
    async fn test_codec_size_limits() {
        let codec = BinaryCodec::<TestMessage>::with_max_size(100);
        let large_message = TestMessage {
            id: 1,
            text: "x".repeat(1000), // Large text
            data: vec![0; 1000],    // Large data
        };

        // Should fail due to size limit
        let result = codec.encode(&large_message).await;
        assert!(matches!(result, Err(NetworkError::MessageTooLarge { .. })));
    }

    #[tokio::test]
    async fn test_version_negotiation() {
        let codec = VersionedCodec::<TestMessage>::new(1)
            .add_version(1, Box::new(BinaryCodec::new()))
            .add_version(2, Box::new(JsonCodec::new()))
            .add_version(3, Box::new(BinaryCodec::new()));

        // Test negotiation
        assert_eq!(codec.negotiate_version(&[1, 2]), Some(2)); // Highest common
        assert_eq!(codec.negotiate_version(&[1, 4]), Some(1)); // Only 1 is common
        assert_eq!(codec.negotiate_version(&[4, 5]), None);    // No common versions

        // Test supported versions
        let supported = codec.supported_versions();
        assert_eq!(supported, vec![1, 2, 3]);
    }
}