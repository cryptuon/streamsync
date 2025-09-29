use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sharding_core::{
    ConsistentHashRing, HashFunctionType, NodeId, ShardConfig, ShardManager,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

fn bench_hash_ring_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_ring");

    // Benchmark adding nodes
    group.bench_function("add_node", |b| {
        b.iter(|| {
            let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);
            for i in 0..100 {
                let node_id = NodeId::new(format!("node-{}", i));
                black_box(ring.add_node(node_id, 10).unwrap());
            }
        })
    });

    // Benchmark key lookups
    let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);
    for i in 0..50 {
        let node_id = NodeId::new(format!("node-{}", i));
        ring.add_node(node_id, 10).unwrap();
    }

    group.bench_function("get_node", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let key = format!("key-{}", i);
                black_box(ring.get_node(&key).unwrap());
            }
        })
    });

    // Benchmark multi-node lookups for replication
    group.bench_function("get_nodes_replication", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let key = format!("key-{}", i);
                black_box(ring.get_nodes(&key, 3).unwrap());
            }
        })
    });

    group.finish();
}

fn bench_hash_functions(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_functions");

    let test_keys: Vec<String> = (0..1000).map(|i| format!("test-key-{}", i)).collect();

    // Benchmark AHash
    group.bench_function("ahash", |b| {
        let ring = ConsistentHashRing::new(HashFunctionType::AHash);
        b.iter(|| {
            for key in &test_keys {
                black_box(ring.hash_function_name());
            }
        })
    });

    // Benchmark SHA-256
    group.bench_function("sha256", |b| {
        let ring = ConsistentHashRing::new(HashFunctionType::Sha256);
        b.iter(|| {
            for key in &test_keys {
                black_box(ring.hash_function_name());
            }
        })
    });

    // Benchmark xxHash
    group.bench_function("xxhash", |b| {
        let ring = ConsistentHashRing::new(HashFunctionType::XxHash);
        b.iter(|| {
            for key in &test_keys {
                black_box(ring.hash_function_name());
            }
        })
    });

    group.finish();
}

fn bench_virtual_nodes_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("virtual_nodes_scaling");

    for virtual_nodes in [10, 50, 100, 200, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("lookup_performance", virtual_nodes),
            virtual_nodes,
            |b, &virtual_nodes| {
                let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);

                // Add nodes with varying virtual node counts
                for i in 0..10 {
                    let node_id = NodeId::new(format!("node-{}", i));
                    ring.add_node(node_id, virtual_nodes).unwrap();
                }

                b.iter(|| {
                    for i in 0..100 {
                        let key = format!("key-{}", i);
                        black_box(ring.get_node(&key).unwrap());
                    }
                })
            },
        );
    }

    group.finish();
}

fn bench_node_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("node_scaling");

    for node_count in [10, 50, 100, 200, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("lookup_with_node_count", node_count),
            node_count,
            |b, &node_count| {
                let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);

                // Add specified number of nodes
                for i in 0..*node_count {
                    let node_id = NodeId::new(format!("node-{}", i));
                    ring.add_node(node_id, 50).unwrap();
                }

                b.iter(|| {
                    for i in 0..100 {
                        let key = format!("key-{}", i);
                        black_box(ring.get_node(&key).unwrap());
                    }
                })
            },
        );
    }

    group.finish();
}

fn bench_shard_manager_operations(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("shard_manager");

    group.bench_function("add_remove_nodes", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = ShardConfig::test_config();
            let manager = ShardManager::new(config);

            // Add nodes
            for i in 0..10 {
                let node_id = NodeId::new(format!("node-{}", i));
                let addr: SocketAddr = format!("127.0.0.1:{}", 8080 + i).parse().unwrap();
                black_box(manager.add_node(node_id, addr).await.unwrap());
            }

            // Get responsible nodes for keys
            for i in 0..100 {
                let key = format!("key-{}", i);
                black_box(manager.get_responsible_nodes(&key).await);
            }

            // Remove some nodes
            for i in 0..5 {
                let node_id = NodeId::new(format!("node-{}", i));
                black_box(manager.remove_node(&node_id).await.unwrap());
            }
        })
    });

    group.finish();
}

fn bench_key_distribution(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_distribution");
    group.throughput(Throughput::Elements(10000));

    group.bench_function("distribution_uniformity", |b| {
        let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);

        // Add nodes
        for i in 0..10 {
            let node_id = NodeId::new(format!("node-{}", i));
            ring.add_node(node_id, 100).unwrap();
        }

        b.iter(|| {
            let mut distribution = HashMap::new();

            // Test key distribution
            for i in 0..10000 {
                let key = format!("key-{}", i);
                let node = ring.get_node(&key).unwrap();
                *distribution.entry(node).or_insert(0) += 1;
            }

            black_box(distribution);
        })
    });

    group.finish();
}

fn bench_rebalancing_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("rebalancing");

    group.bench_function("node_join_impact", |b| {
        b.iter(|| {
            let mut ring = ConsistentHashRing::new(HashFunctionType::AHash);

            // Initial cluster
            for i in 0..20 {
                let node_id = NodeId::new(format!("node-{}", i));
                ring.add_node(node_id, 50).unwrap();
            }

            // Capture initial distribution
            let mut initial_mapping = HashMap::new();
            for i in 0..1000 {
                let key = format!("key-{}", i);
                let node = ring.get_node(&key).unwrap();
                initial_mapping.insert(key, node);
            }

            // Add new node
            let new_node = NodeId::new("new-node");
            let affected_ranges = ring.get_affected_ranges(&[100, 200, 300]);
            ring.add_node(new_node, 50).unwrap();

            // Check how many keys moved
            let mut moved_keys = 0;
            for (key, old_node) in initial_mapping {
                let new_node = ring.get_node(&key).unwrap();
                if old_node != new_node {
                    moved_keys += 1;
                }
            }

            black_box((affected_ranges, moved_keys));
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_hash_ring_operations,
    bench_hash_functions,
    bench_virtual_nodes_scaling,
    bench_node_scaling,
    bench_shard_manager_operations,
    bench_key_distribution,
    bench_rebalancing_simulation
);

criterion_main!(benches);