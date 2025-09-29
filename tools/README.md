# StreamSync Data Collection Tools

This directory contains tools for collecting and analyzing real Solana blockchain data to build comprehensive datasets for testing and validation.

## Tools Overview

### 🔍 Data Collector (`data_collector.rs`)

Collects authentic Solana blockchain data by scanning recent blocks and extracting transactions from target programs:

**Target Programs:**
- **SPL Token**: Standard token operations (transfers, mints, burns)
- **Metaplex Bubblegum**: Compressed NFTs (cNFTs) with state compression
- **Account Compression**: SPL Account Compression for merkle trees
- **Jupiter Aggregator**: DEX aggregation and token swaps

**Features:**
- ✅ Real-time blockchain scanning
- ✅ State compression detection
- ✅ Merkle tree extraction
- ✅ Transaction filtering and validation
- ✅ Configurable collection limits
- ✅ Structured data export (JSON)

### 📊 Dataset Analyzer (`dataset_analyzer.rs`)

Analyzes collected data to validate quality, extract patterns, and generate test recommendations:

**Analysis Features:**
- ✅ Data quality metrics (completeness, success rates)
- ✅ Program pattern analysis (instruction discriminators, account usage)
- ✅ Compression analysis (ratios, merkle trees, proof sizes)
- ✅ Test case recommendations by category
- ✅ Confidence building plans with validation milestones

## Usage

### 1. Collect Real Solana Data

```bash
# Collect data with default configuration
cargo run --bin data_collector

# This will:
# - Scan the latest 1000 blocks on Solana mainnet
# - Extract up to 1000 transactions from target programs
# - Save results to ./collected_data/
```

### 2. Analyze the Dataset

```bash
# Analyze collected data
cargo run --bin dataset_analyzer ./collected_data

# This will:
# - Load and validate the dataset
# - Generate quality metrics and pattern analysis
# - Create test recommendations
# - Output analysis to ./collected_data/dataset_analysis.json
```

## Configuration

### Data Collection Config

```json
{
  "rpc_url": "https://api.mainnet-beta.solana.com",
  "output_directory": "./collected_data",
  "collection_limits": {
    "max_blocks_to_scan": 1000,
    "max_transactions_per_program": 200,
    "max_total_transactions": 1000,
    "max_collection_time_minutes": 30
  },
  "data_filters": {
    "min_transaction_size": 100,
    "max_transaction_size": 10000,
    "include_failed_transactions": false,
    "require_state_compression": false
  }
}
```

## Output Structure

### Collected Data (`solana_dataset.json`)

```json
[
  {
    "signature": "3x7K8j...",
    "slot": 123456789,
    "program_name": "SPL Token",
    "instruction_data": [3, 0, 64, 66, 15, 0, 0, 0, 0, 0],
    "accounts": ["TokenkegQ...", "9WzDXw..."],
    "success": true,
    "is_state_compression": false,
    "merkle_tree": null
  }
]
```

### Analysis Results (`dataset_analysis.json`)

```json
{
  "dataset_quality": {
    "total_transactions": 850,
    "success_rate": 0.94,
    "data_completeness": 0.97,
    "program_coverage": {
      "SPL Token": 320,
      "Metaplex Bubblegum": 180,
      "Jupiter Aggregator": 250
    }
  },
  "test_case_recommendations": [
    {
      "category": "BasicReconstruction",
      "description": "Test basic ZK reconstruction with simple SPL Token transfers",
      "confidence_level": 0.9,
      "sample_transactions": ["3x7K8j...", "4y9L2m..."]
    }
  ]
}
```

## Building Confidence with Real Data

The tools are designed to systematically build confidence in the StreamSync libraries:

### 📈 **Confidence Building Plan**

1. **Minimum Test Cases**: 50+ transactions across all categories
2. **Program Coverage**: 80%+ of target programs represented
3. **Success Criteria**: 90%+ success rate on basic reconstructions

### 🎯 **Validation Milestones**

- **Basic Functionality**: 90% success on simple reconstructions
- **Compression Support**: 70% success on cNFT reconstructions
- **Production Ready**: 95% success across all categories

### 📊 **Quality Metrics**

- **Data Completeness**: Ensure all required fields are populated
- **Temporal Coverage**: Span multiple slots/time periods
- **Size Distribution**: Cover various transaction sizes
- **Failure Analysis**: Include failed transactions for error testing

## Integration with Testing

The collected data integrates directly with the StreamSync test suite:

```rust
// Load real dataset for testing
let dataset = load_collected_dataset("./collected_data/solana_dataset.json")?;

// Test ZK reconstruction with real SPL Token data
for tx in dataset.filter(|t| t.program_name == "SPL Token") {
    let result = zk_reconstructor.reconstruct(&tx.truncated_data).await?;
    assert!(result.confidence_score > 0.7);
}
```

## Benefits of Real Data Testing

✅ **Realistic Patterns**: Test with actual blockchain transaction structures
✅ **Edge Cases**: Discover real-world edge cases not covered by synthetic data
✅ **Performance**: Validate performance with production-sized data
✅ **Confidence**: Build statistical confidence through volume testing
✅ **Compatibility**: Ensure compatibility with real Solana program behaviors
✅ **Validation**: Cross-validate library outputs against known good data

This comprehensive approach ensures that StreamSync libraries work reliably with real Solana blockchain data, building the confidence needed for production deployment.