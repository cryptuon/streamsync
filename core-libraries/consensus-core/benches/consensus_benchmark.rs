//! Benchmarks for consensus performance

use consensus_core::{ConsensusEngine, Config, Proposal};
use consensus_core::transport::InMemoryTransport;
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Benchmark single proposal consensus
fn bench_single_proposal(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("single_proposal", |b| {
        b.to_async(&rt).iter(|| async {
            let node_ids: Vec<_> = (0..4).map(|_| Uuid::new_v4()).collect();
            let transports = InMemoryTransport::create_network(node_ids.clone());

            let mut engines = Vec::new();
            for &node_id in &node_ids {
                let config = Config::new(node_id, node_ids.clone())
                    .with_request_timeout(Duration::from_secs(1));
                let transport = transports.get(&node_id).unwrap().clone();
                let engine = ConsensusEngine::new(config, transport).await.unwrap();
                engines.push(engine);
            }

            for engine in &engines {
                engine.start().await.unwrap();
            }

            let proposal = Proposal::new("bench".to_string(), vec![0u8; 1024]); // 1KB payload
            let result = engines[0].propose(black_box(proposal)).await.unwrap();

            for engine in &engines {
                engine.stop().await.unwrap();
            }

            black_box(result)
        });
    });
}

/// Benchmark multiple sequential proposals
fn bench_sequential_proposals(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("sequential_proposals");

    for num_proposals in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("proposals", num_proposals),
            num_proposals,
            |b, &num_proposals| {
                b.to_async(&rt).iter(|| async move {
                    let node_ids: Vec<_> = (0..4).map(|_| Uuid::new_v4()).collect();
                    let transports = InMemoryTransport::create_network(node_ids.clone());

                    let mut engines = Vec::new();
                    for &node_id in &node_ids {
                        let config = Config::new(node_id, node_ids.clone())
                            .with_request_timeout(Duration::from_secs(5));
                        let transport = transports.get(&node_id).unwrap().clone();
                        let engine = ConsensusEngine::new(config, transport).await.unwrap();
                        engines.push(engine);
                    }

                    for engine in &engines {
                        engine.start().await.unwrap();
                    }

                    let primary = &engines[0];
                    for i in 0..num_proposals {
                        let proposal = Proposal::new(
                            format!("bench_{}", i),
                            vec![i as u8; 1024]
                        );
                        primary.propose(black_box(proposal)).await.unwrap();
                    }

                    for engine in &engines {
                        engine.stop().await.unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark different network sizes
fn bench_network_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("network_sizes");

    for num_nodes in [4, 7, 10, 13].iter() {
        group.bench_with_input(
            BenchmarkId::new("nodes", num_nodes),
            num_nodes,
            |b, &num_nodes| {
                b.to_async(&rt).iter(|| async move {
                    let node_ids: Vec<_> = (0..num_nodes).map(|_| Uuid::new_v4()).collect();
                    let transports = InMemoryTransport::create_network(node_ids.clone());

                    let mut engines = Vec::new();
                    for &node_id in &node_ids {
                        let config = Config::new(node_id, node_ids.clone())
                            .with_request_timeout(Duration::from_secs(2));
                        let transport = transports.get(&node_id).unwrap().clone();
                        let engine = ConsensusEngine::new(config, transport).await.unwrap();
                        engines.push(engine);
                    }

                    for engine in &engines {
                        engine.start().await.unwrap();
                    }

                    let proposal = Proposal::new("bench".to_string(), vec![0u8; 1024]);
                    let result = engines[0].propose(black_box(proposal)).await.unwrap();

                    for engine in &engines {
                        engine.stop().await.unwrap();
                    }

                    black_box(result)
                });
            },
        );
    }
    group.finish();
}

/// Benchmark different payload sizes
fn bench_payload_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("payload_sizes");

    for size in [64, 256, 1024, 4096, 16384].iter() {
        group.bench_with_input(
            BenchmarkId::new("bytes", size),
            size,
            |b, &size| {
                b.to_async(&rt).iter(|| async move {
                    let node_ids: Vec<_> = (0..4).map(|_| Uuid::new_v4()).collect();
                    let transports = InMemoryTransport::create_network(node_ids.clone());

                    let mut engines = Vec::new();
                    for &node_id in &node_ids {
                        let config = Config::new(node_id, node_ids.clone())
                            .with_request_timeout(Duration::from_secs(2));
                        let transport = transports.get(&node_id).unwrap().clone();
                        let engine = ConsensusEngine::new(config, transport).await.unwrap();
                        engines.push(engine);
                    }

                    for engine in &engines {
                        engine.start().await.unwrap();
                    }

                    let proposal = Proposal::new("bench".to_string(), vec![0u8; size]);
                    let result = engines[0].propose(black_box(proposal)).await.unwrap();

                    for engine in &engines {
                        engine.stop().await.unwrap();
                    }

                    black_box(result)
                });
            },
        );
    }
    group.finish();
}

/// Benchmark view change performance
fn bench_view_change(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("view_change", |b| {
        b.to_async(&rt).iter(|| async {
            let node_ids: Vec<_> = (0..4).map(|_| Uuid::new_v4()).collect();
            let transports = InMemoryTransport::create_network(node_ids.clone());

            let mut engines = Vec::new();
            for &node_id in &node_ids {
                let config = Config::new(node_id, node_ids.clone())
                    .with_view_change_timeout(Duration::from_millis(100));
                let transport = transports.get(&node_id).unwrap().clone();
                let engine = ConsensusEngine::new(config, transport).await.unwrap();
                engines.push(engine);
            }

            for engine in &engines {
                engine.start().await.unwrap();
            }

            // Trigger view change
            engines[1].trigger_view_change().await.unwrap();

            // Wait for view change to complete
            let start = std::time::Instant::now();
            while engines[0].current_view().await == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
                if start.elapsed() > Duration::from_secs(1) {
                    break;
                }
            }

            for engine in &engines {
                engine.stop().await.unwrap();
            }

            black_box(())
        });
    });
}

/// Benchmark checkpoint creation
fn bench_checkpoint(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("checkpoint", |b| {
        b.to_async(&rt).iter(|| async {
            let node_ids: Vec<_> = (0..4).map(|_| Uuid::new_v4()).collect();
            let transports = InMemoryTransport::create_network(node_ids.clone());

            let mut engines = Vec::new();
            for &node_id in &node_ids {
                let config = Config::new(node_id, node_ids.clone())
                    .with_checkpoint_interval(5)
                    .with_request_timeout(Duration::from_secs(1));
                let transport = transports.get(&node_id).unwrap().clone();
                let engine = ConsensusEngine::new(config, transport).await.unwrap();
                engines.push(engine);
            }

            for engine in &engines {
                engine.start().await.unwrap();
            }

            // Commit enough proposals to trigger checkpoint
            for i in 0..6 {
                let proposal = Proposal::new(format!("checkpoint_{}", i), vec![i as u8; 256]);
                engines[0].propose(proposal).await.unwrap();
            }

            // Force checkpoint
            engines[0].create_checkpoint().await.unwrap();

            for engine in &engines {
                engine.stop().await.unwrap();
            }

            black_box(())
        });
    });
}

/// Benchmark message throughput
fn bench_message_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("message_throughput");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("sustained_load", |b| {
        b.to_async(&rt).iter(|| async {
            let node_ids: Vec<_> = (0..4).map(|_| Uuid::new_v4()).collect();
            let transports = InMemoryTransport::create_network(node_ids.clone());

            let mut engines = Vec::new();
            for &node_id in &node_ids {
                let config = Config::new(node_id, node_ids.clone())
                    .with_request_timeout(Duration::from_secs(10));
                let transport = transports.get(&node_id).unwrap().clone();
                let engine = ConsensusEngine::new(config, transport).await.unwrap();
                engines.push(engine);
            }

            for engine in &engines {
                engine.start().await.unwrap();
            }

            let primary = &engines[0];

            // Sustained proposal load
            let num_proposals = 50;
            for i in 0..num_proposals {
                let proposal = Proposal::new(format!("throughput_{}", i), vec![i as u8; 512]);
                primary.propose(black_box(proposal)).await.unwrap();
            }

            for engine in &engines {
                engine.stop().await.unwrap();
            }

            black_box(num_proposals)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_proposal,
    bench_sequential_proposals,
    bench_network_sizes,
    bench_payload_sizes,
    bench_view_change,
    bench_checkpoint,
    bench_message_throughput
);

criterion_main!(benches);