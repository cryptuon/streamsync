use anyhow::{anyhow, Result};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use tracing::{debug, warn};

use crate::config::CompressionType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    pub original_size: u64,
    pub compressed_size: u64,
    pub compression_ratio: f64,
    pub compression_time_ms: u128,
    pub decompression_time_ms: u128,
}

#[derive(Clone)]
pub struct CompressionEngine {
    algorithm: CompressionType,
    level: u32,
}

impl CompressionEngine {
    pub fn new(algorithm: CompressionType) -> Self {
        let level = match algorithm {
            CompressionType::None => 0,
            CompressionType::Gzip => 6,
            CompressionType::Zstd => 3,
            CompressionType::Lz4 => 1,
        };

        Self { algorithm, level }
    }

    pub fn with_level(mut self, level: u32) -> Self {
        self.level = level;
        self
    }

    pub fn compress(&self, data: &[u8]) -> Result<(Vec<u8>, CompressionStats)> {
        let start_time = std::time::Instant::now();
        let original_size = data.len() as u64;

        let compressed_data = match self.algorithm {
            CompressionType::None => {
                return Ok((
                    data.to_vec(),
                    CompressionStats {
                        original_size,
                        compressed_size: original_size,
                        compression_ratio: 1.0,
                        compression_time_ms: 0,
                        decompression_time_ms: 0,
                    },
                ));
            }
            CompressionType::Gzip => self.compress_gzip(data)?,
            CompressionType::Zstd => {
                // For simplicity, fall back to gzip
                warn!("Zstd not implemented, falling back to gzip");
                self.compress_gzip(data)?
            }
            CompressionType::Lz4 => {
                // For simplicity, fall back to gzip
                warn!("Lz4 not implemented, falling back to gzip");
                self.compress_gzip(data)?
            }
        };

        let compression_time = start_time.elapsed().as_millis();
        let compressed_size = compressed_data.len() as u64;
        let compression_ratio = if original_size > 0 {
            compressed_size as f64 / original_size as f64
        } else {
            1.0
        };

        debug!(
            "Compressed {} bytes to {} bytes (ratio: {:.2})",
            original_size, compressed_size, compression_ratio
        );

        Ok((
            compressed_data,
            CompressionStats {
                original_size,
                compressed_size,
                compression_ratio,
                compression_time_ms: compression_time,
                decompression_time_ms: 0,
            },
        ))
    }

    pub fn decompress(&self, data: &[u8], expected_size: Option<usize>) -> Result<(Vec<u8>, CompressionStats)> {
        let start_time = std::time::Instant::now();
        let compressed_size = data.len() as u64;

        let decompressed_data = match self.algorithm {
            CompressionType::None => data.to_vec(),
            CompressionType::Gzip => self.decompress_gzip(data)?,
            CompressionType::Zstd => {
                // For simplicity, fall back to gzip
                self.decompress_gzip(data)?
            }
            CompressionType::Lz4 => {
                // For simplicity, fall back to gzip
                self.decompress_gzip(data)?
            }
        };

        let decompression_time = start_time.elapsed().as_millis();
        let original_size = decompressed_data.len() as u64;

        // Validate expected size if provided
        if let Some(expected) = expected_size {
            if decompressed_data.len() != expected {
                return Err(anyhow!(
                    "Decompressed size {} doesn't match expected size {}",
                    decompressed_data.len(),
                    expected
                ));
            }
        }

        debug!(
            "Decompressed {} bytes to {} bytes",
            compressed_size, original_size
        );

        Ok((
            decompressed_data,
            CompressionStats {
                original_size,
                compressed_size,
                compression_ratio: if original_size > 0 {
                    compressed_size as f64 / original_size as f64
                } else {
                    1.0
                },
                compression_time_ms: 0,
                decompression_time_ms: decompression_time,
            },
        ))
    }

    pub fn compress_json(&self, value: &serde_json::Value) -> Result<(Vec<u8>, CompressionStats)> {
        let json_bytes = serde_json::to_vec(value)?;
        self.compress(&json_bytes)
    }

    pub fn decompress_json(&self, data: &[u8]) -> Result<(serde_json::Value, CompressionStats)> {
        let (decompressed_data, stats) = self.decompress(data, None)?;
        let json_value = serde_json::from_slice(&decompressed_data)?;
        Ok((json_value, stats))
    }

    fn compress_gzip(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.level));
        encoder.write_all(data)?;
        let compressed = encoder.finish()?;
        Ok(compressed)
    }

    fn decompress_gzip(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }

    pub fn estimate_compression_ratio(&self, sample_data: &[u8]) -> Result<f64> {
        if sample_data.len() < 100 {
            return Ok(1.0); // No compression benefit for very small data
        }

        // Take a sample of the data to estimate compression
        let sample_size = std::cmp::min(sample_data.len(), 1024);
        let sample = &sample_data[..sample_size];

        let (_, stats) = self.compress(sample)?;
        Ok(stats.compression_ratio)
    }
}

// Utility functions for batch compression
#[derive(Clone)]
pub struct BatchCompressor {
    engine: CompressionEngine,
    batch_size: usize,
}

impl BatchCompressor {
    pub fn new(algorithm: CompressionType, batch_size: usize) -> Self {
        Self {
            engine: CompressionEngine::new(algorithm),
            batch_size,
        }
    }

    pub async fn compress_records(&self, records: Vec<serde_json::Value>) -> Result<Vec<u8>> {
        // Serialize records to JSON array
        let json_data = serde_json::to_vec(&records)?;

        // Compress the serialized data
        let (compressed_data, stats) = self.engine.compress(&json_data)?;

        debug!(
            "Compressed {} records: {} -> {} bytes (ratio: {:.2})",
            records.len(),
            stats.original_size,
            stats.compressed_size,
            stats.compression_ratio
        );

        Ok(compressed_data)
    }

    pub async fn decompress_records(&self, compressed_data: &[u8]) -> Result<Vec<serde_json::Value>> {
        // Decompress the data
        let (decompressed_data, _stats) = self.engine.decompress(compressed_data, None)?;

        // Deserialize from JSON array
        let records: Vec<serde_json::Value> = serde_json::from_slice(&decompressed_data)?;

        debug!("Decompressed {} records", records.len());
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compression_roundtrip() {
        let engine = CompressionEngine::new(CompressionType::Gzip);
        let original_data = b"Hello, world! This is some test data for compression.";

        let (compressed, compress_stats) = engine.compress(original_data).unwrap();
        let (decompressed, _decompress_stats) = engine.decompress(&compressed, Some(original_data.len())).unwrap();

        assert_eq!(original_data.as_ref(), decompressed.as_slice());
        assert!(compress_stats.compressed_size <= compress_stats.original_size);
    }

    #[tokio::test]
    async fn test_json_compression() {
        let engine = CompressionEngine::new(CompressionType::Gzip);
        let json_data = serde_json::json!({
            "name": "test",
            "data": vec![1, 2, 3, 4, 5],
            "nested": {
                "key": "value"
            }
        });

        let (compressed, _stats) = engine.compress_json(&json_data).unwrap();
        let (decompressed, _stats) = engine.decompress_json(&compressed).unwrap();

        assert_eq!(json_data, decompressed);
    }

    #[tokio::test]
    async fn test_batch_compression() {
        let compressor = BatchCompressor::new(CompressionType::Gzip, 100);
        let records = vec![
            serde_json::json!({"id": 1, "name": "Alice"}),
            serde_json::json!({"id": 2, "name": "Bob"}),
            serde_json::json!({"id": 3, "name": "Charlie"}),
        ];

        let compressed = compressor.compress_records(records.clone()).await.unwrap();
        let decompressed = compressor.decompress_records(&compressed).await.unwrap();

        assert_eq!(records, decompressed);
    }
}