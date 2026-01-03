# Getting Started: Development Environment Setup

Instructions for setting up the development environment and building StreamSync.

## Quick Start

```bash
# Clone and build
git clone https://github.com/your-org/streamsync.git
cd streamsync

# Build everything
cargo build --release

# Run all tests (193+ passing)
cargo test --workspace

# Run a specific library's tests
cargo test --package networking-core   # 45 tests
cargo test --package sharding-core     # 60 tests
cargo test --package distributed-duckdb # 34 tests
```

## Prerequisites

### Hardware Requirements
```bash
# Minimum development setup
CPU: 8 cores (16 recommended)
RAM: 32GB (64GB recommended)
Storage: 500GB SSD
Network: 100Mbps connection

# Production node requirements
CPU: 32+ cores
RAM: 128GB+ (256GB for reconstruction specialists)
Storage: 4TB+ NVMe SSD
Network: 10Gbps connection
```

### Software Requirements
```bash
# Core development tools
Rust 1.75+          # cargo, rustc, rustfmt, clippy
Git 2.0+
OpenSSL development headers

# Solana tools (for token program)
Solana CLI 1.16+
Anchor CLI 0.30+

# Optional but recommended
Docker & Docker Compose  # For local testing
Redis                    # For caching layer
```

## Development Environment Setup

### 1. Repository Setup
```bash
# Clone the repository
git clone https://github.com/your-org/streamsync.git
cd streamsync

# Install Rust toolchain
rustup update stable
rustup default stable
rustup component add clippy rustfmt

# Install system dependencies (Ubuntu/Debian)
sudo apt update
sudo apt install -y \
    build-essential \
    libssl-dev \
    pkg-config \
    protobuf-compiler

# Install system dependencies (macOS)
brew install openssl protobuf

# Verify installation
cargo --version
rustc --version
```

### 2. Build All Libraries
```bash
# Build the entire workspace
cargo build --release

# This builds:
# - streamsync (main binary)
# - All 8 core libraries
# - Tools and examples
```

### 3. Run Tests
```bash
# Run all tests (193+ tests)
cargo test --workspace

# Test output summary:
# networking-core:     45 passed
# sharding-core:       60 passed
# distributed-duckdb:  34 passed
# idl-sync:            18 passed
# zk-reconstruction:    8 passed
# solana-indexer:       6 passed
# storage-core:         3 passed
# program-parser:       8 passed
# + doc tests and integration tests
```

### 3. Network Node Setup
```bash
# Build network node
cd network-node
cargo build --release

# Generate node configuration
./target/release/network-node generate-config \
    --node-type query-node \
    --specialization general \
    --data-dir ./data \
    --config-out ./node-config.toml

# Initialize local database
./target/release/network-node init-db \
    --config ./node-config.toml \
    --genesis-data ./test-data/genesis.json
```

### 4. Development Network Setup
```bash
# Start local development network with Docker Compose
docker-compose -f docker/dev-network.yml up -d

# This starts:
# - 5 test nodes with different specializations
# - Local Solana test validator
# - PostgreSQL for metadata
# - Redis for caching
# - Grafana/Prometheus for monitoring

# Verify network is running
curl http://localhost:8080/health
curl http://localhost:8081/health
curl http://localhost:8082/health
```

## Core Library Development

### ZK Reconstruction Library
```rust
// Example: Adding a new reconstruction strategy
// File: core-libraries/zk-reconstruction/src/strategies/custom_strategy.rs

use crate::{ReconstructionStrategy, ReconstructionError, ReconstructedAccount};

pub struct CustomReconstructionStrategy {
    // Implementation details
}

impl ReconstructionStrategy for CustomReconstructionStrategy {
    async fn reconstruct(
        &self,
        truncated_data: &[u8],
        compression_params: &CompressionParams
    ) -> Result<ReconstructedAccount, ReconstructionError> {
        // Your reconstruction logic here
        todo!()
    }

    fn can_handle(&self, compression_type: &CompressionType) -> bool {
        // Return true if this strategy can handle the compression type
        matches!(compression_type, CompressionType::Custom(_))
    }

    fn estimated_complexity(&self, truncated_data: &[u8]) -> ComplexityEstimate {
        // Estimate how expensive this reconstruction will be
        ComplexityEstimate::Medium
    }
}

// Register the new strategy
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_custom_reconstruction() {
        let strategy = CustomReconstructionStrategy::new();
        let test_data = load_test_compression_data();

        let result = strategy.reconstruct(&test_data.truncated, &test_data.params).await;
        assert!(result.is_ok());
    }
}
```

### IDL Synchronization Library
```rust
// Example: Adding support for a new program pattern
// File: core-libraries/idl-sync/src/analyzers/custom_program_analyzer.rs

use crate::{ProgramAnalyzer, InstructionPattern, AnalysisError};

pub struct CustomProgramAnalyzer {
    pattern_matcher: PatternMatcher,
}

impl ProgramAnalyzer for CustomProgramAnalyzer {
    async fn analyze_program_behavior(
        &self,
        program_id: &Pubkey,
        transactions: &[Transaction]
    ) -> Result<Vec<InstructionPattern>, AnalysisError> {

        // Filter transactions for this program
        let program_txs: Vec<_> = transactions.iter()
            .filter(|tx| tx.involves_program(program_id))
            .collect();

        // Apply custom pattern matching
        let patterns = self.pattern_matcher.find_patterns(&program_txs)?;

        Ok(patterns)
    }

    fn supported_program_types(&self) -> Vec<ProgramType> {
        vec![ProgramType::Custom("my-program-type".to_string())]
    }
}

// Testing
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_custom_program_analysis() {
        let analyzer = CustomProgramAnalyzer::new();
        let test_program_id = Pubkey::new_unique();
        let test_transactions = load_test_transactions(&test_program_id);

        let patterns = analyzer.analyze_program_behavior(&test_program_id, &test_transactions).await?;

        assert!(!patterns.is_empty());
        assert!(patterns.iter().all(|p| p.confidence > 0.8));
    }
}
```

### Distributed DuckDB Integration
```rust
// Example: Adding a new query optimization
// File: core-libraries/distributed-duckdb/src/optimizers/custom_optimizer.rs

use crate::{QueryOptimizer, DistributedQuery, OptimizedQuery, OptimizationError};

pub struct CustomQueryOptimizer {
    cost_estimator: CostEstimator,
}

impl QueryOptimizer for CustomQueryOptimizer {
    async fn optimize_query(
        &self,
        query: &DistributedQuery,
        available_nodes: &[NodeInfo]
    ) -> Result<OptimizedQuery, OptimizationError> {

        // Analyze query for optimization opportunities
        let analysis = self.analyze_query_structure(query)?;

        // Estimate costs for different execution strategies
        let execution_options = self.generate_execution_options(&analysis, available_nodes);
        let cost_estimates = self.cost_estimator.estimate_costs(&execution_options).await?;

        // Select best execution strategy
        let best_strategy = cost_estimates.into_iter()
            .min_by_key(|estimate| estimate.total_cost)
            .ok_or(OptimizationError::NoViableStrategy)?;

        Ok(OptimizedQuery {
            execution_plan: best_strategy.execution_plan,
            estimated_cost: best_strategy.total_cost,
            estimated_latency: best_strategy.estimated_latency,
        })
    }
}

// Testing
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_optimization() {
        let optimizer = CustomQueryOptimizer::new();
        let test_query = DistributedQuery::parse("SELECT * FROM accounts WHERE owner = $1")?;
        let test_nodes = generate_test_node_topology(5);

        let optimized = optimizer.optimize_query(&test_query, &test_nodes).await?;

        assert!(optimized.estimated_latency < Duration::from_millis(10));
        assert!(optimized.execution_plan.node_count() <= test_nodes.len());
    }
}
```

## Network Node Development

### Basic Node Implementation
```rust
// Example: Implementing a specialized query node
// File: network-node/src/specialized_nodes/speed_runner.rs

use crate::{NetworkNode, QueryHandler, NodeSpecialization};

pub struct SpeedRunnerNode {
    // Optimized for sub-1ms queries
    hot_cache: LRUCache<QueryHash, CachedResult>,
    local_db: OptimizedDuckDBConnection,
    network_interface: LowLatencyNetworkInterface,
}

impl NetworkNode for SpeedRunnerNode {
    fn specialization(&self) -> NodeSpecialization {
        NodeSpecialization::SpeedRunner {
            target_latency: Duration::from_micros(500),
            cache_capacity_gb: 32,
            supported_query_types: vec![
                QueryType::SimpleAccountLookup,
                QueryType::TokenBalance,
                QueryType::BasicAggregation,
            ],
        }
    }

    async fn handle_query(&self, query: NetworkQuery) -> Result<QueryResult, QueryError> {
        let start_time = Instant::now();

        // Ultra-fast cache check first
        if let Some(cached) = self.hot_cache.get(&query.hash()) {
            if cached.is_fresh(Duration::from_secs(5)) {
                return Ok(cached.result);
            }
        }

        // Optimized local execution
        let result = self.execute_optimized_query(&query).await?;

        // Cache for future queries
        self.hot_cache.insert(query.hash(), CachedResult {
            result: result.clone(),
            timestamp: Instant::now(),
        });

        let latency = start_time.elapsed();

        // Record performance metrics
        self.record_performance_metric(latency, &query).await;

        Ok(result)
    }
}

impl SpeedRunnerNode {
    async fn execute_optimized_query(&self, query: &NetworkQuery) -> Result<QueryResult, QueryError> {
        match &query.query_type {
            QueryType::SimpleAccountLookup { pubkey } => {
                // Optimized path for account lookups
                self.local_db.get_account_optimized(pubkey).await
            },

            QueryType::TokenBalance { owner, mint } => {
                // Optimized path for token balances
                self.local_db.get_token_balance_optimized(owner, mint).await
            },

            _ => {
                // Fall back to general query execution
                self.local_db.execute_general_query(query).await
            }
        }
    }
}
```

### Node Configuration
```toml
# Example node configuration
# File: network-node/configs/speed-runner.toml

[node]
id = "speed-runner-001"
type = "speed-runner"
region = "us-east-1"

[specialization]
target_latency_micros = 500
cache_capacity_gb = 32
supported_query_types = ["simple_account_lookup", "token_balance", "basic_aggregation"]

[network]
listen_address = "0.0.0.0:8080"
discovery_nodes = [
    "indexing-network-discovery-1.example.com:7878",
    "indexing-network-discovery-2.example.com:7878"
]
max_connections = 100

[database]
path = "./data/speedrunner.duckdb"
memory_limit = "80%"
threads = 0
cache_size_mb = 2048

[performance]
query_timeout_ms = 15
cache_ttl_seconds = 5
metrics_interval_seconds = 10

[economics]
stake_account = "your-stake-account-pubkey"
reward_address = "your-reward-address-pubkey"
```

## Testing Framework

### Integration Testing
```bash
# Run full integration test suite
cd integration-tests

# Start test network
./scripts/start-test-network.sh

# Run performance tests
cargo test --release performance_tests -- --ignored

# Run consensus tests
cargo test --release consensus_tests

# Run economic simulation tests
cargo test --release economic_simulation_tests

# Clean up test network
./scripts/stop-test-network.sh
```

### Load Testing
```bash
# Install load testing tools
cargo install --git https://github.com/your-org/query-load-tester.git

# Run load tests against development network
query-load-tester \
    --target http://localhost:8080 \
    --queries-per-second 1000 \
    --duration 300s \
    --query-types simple_account_lookup,token_balance \
    --report-file load-test-results.json

# Analyze results
cargo run --bin analyze-load-test -- load-test-results.json
```

### Benchmark Suite
```rust
// Example: Performance benchmarks
// File: benchmarks/src/query_performance.rs

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_query_execution(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let test_network = rt.block_on(setup_test_network());

    let query_types = vec![
        ("simple_account", generate_simple_account_query()),
        ("token_balance", generate_token_balance_query()),
        ("complex_aggregation", generate_complex_aggregation_query()),
    ];

    let mut group = c.benchmark_group("query_execution");

    for (name, query) in query_types {
        group.bench_with_input(
            BenchmarkId::new("distributed", name),
            &query,
            |b, query| {
                b.to_async(&rt).iter(|| {
                    test_network.execute_query(query.clone())
                });
            }
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_query_execution);
criterion_main!(benches);
```

## Development Workflow

### 1. Daily Development
```bash
# Pull latest changes
git pull origin main

# Run pre-commit checks
cargo fmt --all
cargo clippy --all-targets --all-features
cargo test --all

# Build development network
docker-compose -f docker/dev-network.yml up -d

# Run integration tests
cd integration-tests && cargo test

# Develop your feature...

# Before committing
cargo test --all
./scripts/run-performance-tests.sh
```

### 2. Adding New Features
```bash
# Create feature branch
git checkout -b feature/new-query-type

# Implement in appropriate library
cd core-libraries/distributed-duckdb
# ... make changes ...

# Add tests
cargo test new_query_type_tests

# Test with integration suite
cd ../../integration-tests
cargo test test_new_query_type_integration

# Performance validation
./scripts/benchmark-new-feature.sh

# Create pull request
git commit -am "Add support for new query type"
git push origin feature/new-query-type
```

### 3. Performance Optimization
```bash
# Profile query execution
cargo install flamegraph
cargo flamegraph --bin network-node -- --config speed-runner.toml

# Run detailed benchmarks
cd benchmarks
cargo bench --bench query_performance

# Analyze results
cargo run --bin performance-analyzer -- benchmarks/results/

# Validate optimizations don't break correctness
cargo test --release --all
```

## Production Deployment Preparation

### Build Production Binaries
```bash
# Build optimized production binaries
cargo build --release --bin network-node
cargo build --release --bin settlement-engine
cargo build --release --bin monitoring-agent

# Create deployment package
./scripts/create-deployment-package.sh --version 1.0.0
```

### Configuration Management
```bash
# Generate production configurations
./scripts/generate-prod-configs.sh \
    --node-type query-node \
    --region us-east-1 \
    --stake-account <your-stake-pubkey> \
    --output-dir ./prod-configs/
```

This development setup provides everything needed to:
- **Build and test** the core libraries
- **Run a development network** locally
- **Add new features** with proper testing
- **Optimize performance** with benchmarking tools
- **Prepare for production** deployment

The architecture supports rapid development while maintaining the performance guarantees required for production use.