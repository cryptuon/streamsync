# Economic Incentive Model

How to align individual node incentives with network-wide performance goals while ensuring long-term sustainability.

## Core Principle: Performance-Based Economics

Unlike traditional blockchain networks that reward consensus participation, this network rewards **performance outcomes**:

- **Speed**: Faster query responses earn more
- **Accuracy**: Correct reconstructions and predictions earn more
- **Efficiency**: Better cache hit ratios and resource utilization earn more
- **Reliability**: Consistent uptime and performance earn reputation bonuses

## Multi-Token Economic Model

### Primary Token: QUERY ($QUERY)
Used for query payments and basic network operations:

```rust
pub struct QueryPayment {
    base_fee: u64,              // Base cost per query type
    performance_multiplier: f64, // Bonus for sub-target response times
    complexity_factor: f64,      // Additional cost for expensive queries
    priority_bonus: u64,         // Optional expedited processing
}

impl QueryPayment {
    pub fn calculate_total_cost(&self) -> u64 {
        let base_cost = self.base_fee as f64;
        let performance_cost = base_cost * self.performance_multiplier;
        let complexity_cost = performance_cost * self.complexity_factor;

        (complexity_cost as u64) + self.priority_bonus
    }
}
```

### Secondary Token: COMPUTE ($COMP)
Rewards for expensive computational work (ZK reconstruction, ML training):

```rust
pub enum ComputeReward {
    ZKReconstruction {
        difficulty: ReconstructionDifficulty,
        accuracy_verified: bool,
        computation_time: Duration,
    },
    MLModelTraining {
        model_type: ModelType,
        improvement_score: f64,
        training_cost: ComputeUnits,
    },
    IDLBehaviorAnalysis {
        transactions_analyzed: u64,
        patterns_discovered: u32,
        consensus_agreement: f64,
    },
}

impl ComputeReward {
    pub fn calculate_comp_reward(&self) -> u64 {
        match self {
            ComputeReward::ZKReconstruction { difficulty, accuracy_verified, .. } => {
                let base_reward = difficulty.base_reward();
                if *accuracy_verified {
                    base_reward * 2 // Double reward for verified accuracy
                } else {
                    base_reward / 2 // Reduced reward pending verification
                }
            },
            // ... other reward calculations
        }
    }
}
```

## Reward Categories and Mechanisms

### 1. Query Serving Rewards
Direct rewards for handling user queries:

```rust
pub struct QueryServingRewards {
    // Base reward per query served
    base_query_reward: u64,

    // Performance bonuses
    speed_bonus_tiers: Vec<(Duration, f64)>, // (max_latency, multiplier)
    accuracy_bonus: f64,

    // Volume bonuses
    volume_tiers: Vec<(u64, f64)>, // (min_queries_per_hour, multiplier)
}

impl QueryServingRewards {
    pub fn calculate_serving_reward(&self,
        response_time: Duration,
        query_accuracy: f64,
        hourly_volume: u64
    ) -> u64 {
        let base = self.base_query_reward as f64;

        // Speed bonus
        let speed_multiplier = self.speed_bonus_tiers.iter()
            .find(|(max_latency, _)| response_time <= *max_latency)
            .map(|(_, multiplier)| *multiplier)
            .unwrap_or(1.0);

        // Accuracy bonus
        let accuracy_multiplier = 1.0 + (query_accuracy * self.accuracy_bonus);

        // Volume bonus
        let volume_multiplier = self.volume_tiers.iter()
            .filter(|(min_volume, _)| hourly_volume >= *min_volume)
            .map(|(_, multiplier)| *multiplier)
            .fold(1.0, |acc, m| acc * m);

        (base * speed_multiplier * accuracy_multiplier * volume_multiplier) as u64
    }
}
```

### 2. Cache Contribution Rewards
Rewards for providing valuable caching services:

```rust
pub struct CacheContributionRewards {
    // Rewards for cache hits served to other nodes
    cache_hit_reward: u64,

    // Rewards for successful predictions
    prediction_accuracy_bonus: HashMap<f64, u64>, // accuracy_threshold -> bonus

    // Rewards for cache capacity provision
    capacity_provision_reward: u64, // per GB-hour provided
}

impl CacheContributionRewards {
    pub fn calculate_cache_rewards(&self,
        hits_served: u64,
        prediction_accuracy: f64,
        capacity_provided_gb_hours: f64
    ) -> u64 {
        // Base reward for hits served
        let hit_rewards = hits_served * self.cache_hit_reward;

        // Prediction accuracy bonus
        let prediction_bonus = self.prediction_accuracy_bonus.iter()
            .filter(|(threshold, _)| prediction_accuracy >= **threshold)
            .map(|(_, bonus)| *bonus)
            .sum::<u64>();

        // Capacity provision rewards
        let capacity_rewards = (capacity_provided_gb_hours * self.capacity_provision_reward as f64) as u64;

        hit_rewards + prediction_bonus + capacity_rewards
    }
}
```

### 3. Infrastructure Specialization Rewards
Different reward structures for specialized node types:

```rust
pub enum NodeSpecialization {
    ReconstructionSpecialist {
        compute_power_rating: u64,
        historical_accuracy: f64,
        specialization_bonus: f64,
    },
    CacheOptimizer {
        memory_capacity_gb: u64,
        network_bandwidth_mbps: u64,
        geographic_coverage_bonus: f64,
    },
    EdgeNode {
        latency_rating: Duration,
        geographic_location: Region,
        proximity_bonus: f64,
    },
    ArchiveNode {
        storage_capacity_tb: u64,
        data_retention_period: Duration,
        reliability_rating: f64,
    },
}

impl NodeSpecialization {
    pub fn calculate_specialization_multiplier(&self) -> f64 {
        match self {
            NodeSpecialization::ReconstructionSpecialist { historical_accuracy, specialization_bonus, .. } => {
                // Higher rewards for nodes with proven reconstruction accuracy
                specialization_bonus * historical_accuracy
            },
            NodeSpecialization::CacheOptimizer { geographic_coverage_bonus, .. } => {
                // Bonus for nodes that serve multiple geographic regions
                *geographic_coverage_bonus
            },
            // ... other specialization bonuses
        }
    }
}
```

## Anti-Gaming Mechanisms

### 1. Proof-of-Performance
All performance claims must be cryptographically verifiable:

```rust
pub struct PerformanceProof {
    // Cryptographic proof of query response time
    latency_proof: LatencyAttestation,

    // Merkle proof of query result accuracy
    accuracy_proof: AccuracyProof,

    // Third-party validation signatures
    validator_signatures: Vec<ValidatorSignature>,
}

impl PerformanceProof {
    pub fn verify(&self, public_parameters: &NetworkParams) -> bool {
        // Verify latency measurements from multiple sources
        let latency_valid = self.latency_proof.verify_with_multiple_attestors();

        // Verify accuracy against known ground truth
        let accuracy_valid = self.accuracy_proof.verify_against_chain_state();

        // Verify third-party validator signatures
        let validators_valid = self.validator_signatures.iter()
            .all(|sig| sig.verify(public_parameters));

        latency_valid && accuracy_valid && validators_valid
    }
}
```

### 2. Reputation-Based Scaling
Rewards scale with long-term reputation to prevent Sybil attacks:

```rust
pub struct ReputationSystem {
    performance_history: PerformanceHistory,
    stake_amount: u64,
    time_in_network: Duration,
    consensus_participation: ConsensusMetrics,
}

impl ReputationSystem {
    pub fn calculate_reputation_multiplier(&self) -> f64 {
        let performance_score = self.performance_history.calculate_score();
        let stake_factor = (self.stake_amount as f64).log10() / 10.0; // Logarithmic stake bonus
        let time_factor = self.time_in_network.as_secs() as f64 / (365.0 * 24.0 * 3600.0); // Years in network
        let consensus_factor = self.consensus_participation.accuracy_rate();

        (performance_score * (1.0 + stake_factor) * (1.0 + time_factor) * consensus_factor).min(5.0)
    }
}
```

### 3. Economic Slashing for Misbehavior
Stake slashing for provably incorrect behavior:

```rust
pub enum SlashingCondition {
    IncorrectReconstruction {
        evidence: ConflictingReconstructionProof,
        stake_penalty_percentage: f64,
    },
    FalsePerformanceClaim {
        evidence: PerformanceCounterProof,
        stake_penalty_percentage: f64,
    },
    CachePollustion {
        evidence: InvalidCacheEntryProof,
        stake_penalty_percentage: f64,
    },
    ConsensusViolation {
        evidence: ConsensusViolationProof,
        stake_penalty_percentage: f64,
    },
}

impl SlashingCondition {
    pub fn calculate_penalty(&self, node_stake: u64) -> u64 {
        let penalty_percentage = match self {
            SlashingCondition::IncorrectReconstruction { stake_penalty_percentage, .. } => *stake_penalty_percentage,
            SlashingCondition::FalsePerformanceClaim { stake_penalty_percentage, .. } => *stake_penalty_percentage,
            // ... other conditions
        };

        (node_stake as f64 * penalty_percentage) as u64
    }
}
```

## Bootstrap Economics

### Phase 1: Foundation Incentives (Months 1-6)
High rewards to attract initial high-quality nodes:

```rust
pub struct BootstrapIncentives {
    // Higher base rewards during bootstrap phase
    bootstrap_reward_multiplier: f64, // 5x normal rewards

    // Guaranteed minimum earnings for early participants
    minimum_earning_guarantee: u64,

    // Special rewards for network infrastructure development
    infrastructure_development_rewards: HashMap<MilestoneType, u64>,
}
```

### Phase 2: Growth Incentives (Months 6-18)
Gradually scale rewards based on network adoption:

```rust
pub struct GrowthIncentives {
    // Rewards scale with network usage
    usage_based_scaling: f64,

    // Bonuses for geographic expansion
    geographic_expansion_bonuses: HashMap<Region, u64>,

    // Developer adoption incentives
    api_usage_growth_rewards: f64,
}
```

### Phase 3: Mature Network Economics (Month 18+)
Self-sustaining economics based on query demand:

```rust
pub struct MatureNetworkEconomics {
    // Market-driven pricing for queries
    dynamic_pricing_enabled: bool,

    // Reduced bootstrap subsidies
    subsidy_reduction_schedule: Vec<(Duration, f64)>,

    // Long-term sustainability mechanisms
    treasury_sustainability_rate: f64,
}
```

## Revenue Sources and Sustainability

### Primary Revenue: Query Fees
Users pay for queries based on complexity and performance requirements:

```rust
pub struct QueryPricing {
    simple_query_base_fee: u64,    // Basic account lookups
    complex_query_multiplier: f64, // Complex aggregations
    reconstruction_premium: f64,    // ZK reconstruction queries
    real_time_premium: f64,        // Sub-1ms guarantee queries
}
```

### Secondary Revenue: Data Services
Value-added services beyond basic indexing:

```rust
pub enum ValueAddedService {
    HistoricalAnalytics { fee_per_query: u64 },
    PredictiveModeling { subscription_fee: u64 },
    CustomIDLGeneration { fee_per_program: u64 },
    PerformanceGuarantees { sla_premium: f64 },
}
```

## Economic Governance

### Token Holder Voting
Major economic parameters controlled by token holders:

```rust
pub struct EconomicGovernanceProposal {
    proposal_type: ProposalType,
    proposed_changes: HashMap<String, Value>,
    voting_period: Duration,
    execution_delay: Duration,
}

pub enum ProposalType {
    RewardRateAdjustment,
    SlashingParameterChange,
    NewRewardCategory,
    EconomicParameterUpdate,
}
```

### Automatic Economic Adjustments
Some parameters adjust automatically based on network conditions:

```rust
pub struct AutomaticEconomicAdjustments {
    // Adjust reward rates based on network utilization
    utilization_based_adjustments: bool,

    // Adjust query pricing based on demand
    demand_based_pricing: bool,

    // Adjust slashing rates based on misbehavior frequency
    adaptive_slashing_rates: bool,
}
```

## Open Economic Questions

1. **Optimal Reward Distribution**: What percentage of query fees should go to different node types?

2. **Bootstrap Duration**: How long should enhanced bootstrap rewards last?

3. **Geographic Balancing**: How do we ensure adequate node coverage in all regions?

4. **Compute Cost Recovery**: How do we ensure expensive operations (ZK reconstruction) are economically viable?

5. **Long-term Sustainability**: What happens if query demand doesn't grow as expected?

The goal is creating a self-sustaining economy that rewards performance while ensuring the network remains economically viable for both operators and users.