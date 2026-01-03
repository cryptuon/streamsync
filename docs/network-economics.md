# Network Economics: Market-Driven Performance and Pricing

How competitive markets and economic incentives create better performance at fair prices while eliminating vendor lock-in.

> **See also**: [Token Economics](token-economics.md) for complete $STRM token details, staking requirements, and smart contract architecture.

## Economic Model Overview

### $STRM Token System
```rust
pub enum PaymentToken {
    STRM,           // Native token - lowest fees
    SOL,            // Solana native - convenient
    USDC,           // Stablecoin - predictable costs
    SPL(String),    // Custom SPL tokens
}

pub struct RevenueSharing {
    treasury_percentage: 0.20,        // 20% - protocol development
    node_operator_percentage: 0.50,   // 50% - nodes serving requests
    data_provider_percentage: 0.20,   // 20% - Solana RPC providers
    governance_percentage: 0.10,      // 10% - STRM stakers
}
```

### Customer Economic Flow
```rust
impl CustomerEconomics {
    pub async fn customer_journey(&self) -> EconomicFlow {
        // 1. Customer purchases QUERY tokens with SOL
        let query_tokens = self.purchase_query_tokens(PaymentMethod::SOL(amount)).await;

        // 2. Customer submits queries with performance requirements
        let query_result = self.submit_query(Query {
            data_request: query,
            performance_target: PerformanceTarget::SubTenMillisecond,
            max_payment: query_tokens.amount(100), // Willing to pay 100 QUERY tokens
        }).await;

        // 3. Network executes query with racing nodes
        match query_result {
            QueryResult::Success { latency, data } if latency <= performance_target => {
                // Target met: customer pays agreed amount
                self.deduct_query_tokens(100);
                EconomicFlow::CustomerPaid(data)
            },
            QueryResult::Success { latency, data } if latency > performance_target => {
                // Target missed: customer pays nothing
                self.refund_query_tokens(100);
                EconomicFlow::CustomerRefunded(data)
            },
            QueryResult::Failure => {
                // Query failed: customer pays nothing
                self.refund_query_tokens(100);
                EconomicFlow::CustomerRefunded(Error)
            }
        }
    }
}
```

## Market-Driven Pricing Mechanism

### Dynamic Pricing Based on Supply and Demand
```rust
pub struct MarketPricing {
    // Real-time pricing based on network conditions
    base_pricing: BasePricing,
    supply_demand_multiplier: f64,
    performance_premium: HashMap<PerformanceTarget, f64>,
    specialization_premium: HashMap<NodeSpecialization, f64>,
}

impl MarketPricing {
    pub fn calculate_query_price(&self, query_request: QueryRequest) -> Price {
        let base_price = self.base_pricing.for_query_type(&query_request.query_type);

        // Supply and demand adjustment
        let supply_factor = self.calculate_supply_factor(&query_request);
        let demand_factor = self.calculate_demand_factor(&query_request);
        let market_multiplier = demand_factor / supply_factor;

        // Performance premium
        let performance_multiplier = self.performance_premium
            .get(&query_request.performance_target)
            .unwrap_or(&1.0);

        // Specialization premium (if query requires specific node type)
        let specialization_multiplier = query_request.required_specialization
            .map(|spec| self.specialization_premium.get(&spec).unwrap_or(&1.0))
            .unwrap_or(&1.0);

        Price {
            base: base_price,
            total: base_price * market_multiplier * performance_multiplier * specialization_multiplier,
            breakdown: PriceBreakdown {
                base_price,
                market_adjustment: market_multiplier,
                performance_premium: performance_multiplier,
                specialization_premium: specialization_multiplier,
            }
        }
    }

    fn calculate_supply_factor(&self, query_request: &QueryRequest) -> f64 {
        // How many nodes can handle this query type?
        let capable_nodes = self.count_nodes_capable_of_query(&query_request.query_type);
        let total_capacity = capable_nodes.iter()
            .map(|node| node.current_capacity())
            .sum::<f64>();

        // More available capacity = lower prices
        (total_capacity / 100.0).min(2.0) // Cap at 2x supply factor
    }

    fn calculate_demand_factor(&self, query_request: &QueryRequest) -> f64 {
        // How much demand is there for this query type right now?
        let recent_query_volume = self.get_recent_query_volume(&query_request.query_type);
        let historical_average = self.get_historical_average(&query_request.query_type);

        // Higher than normal demand = higher prices
        (recent_query_volume / historical_average).max(0.5) // Floor at 0.5x demand factor
    }
}
```

### Price Discovery Examples
```rust
// Example 1: High demand, low supply = higher prices
let rush_hour_pricing = MarketPricing::calculate_price(QueryRequest {
    query_type: SimpleAccountLookup,
    performance_target: SubMillisecond,
    time: PeakTraffic, // DeFi rush hour
});
// Result: 150% of base price due to demand spike

// Example 2: Low demand, high supply = lower prices
let off_peak_pricing = MarketPricing::calculate_price(QueryRequest {
    query_type: HistoricalAnalysis,
    performance_target: BestEffort,
    time: OffPeak, // 3 AM
});
// Result: 60% of base price due to excess capacity

// Example 3: Specialized requirement = premium pricing
let specialized_pricing = MarketPricing::calculate_price(QueryRequest {
    query_type: ZKReconstruction,
    performance_target: SubTenMillisecond,
    specialization_required: ReconstructionSpecialist,
});
// Result: 200% of base price due to specialized requirement
```

## Node Operator Economics

### Revenue Opportunities
```rust
pub struct NodeOperatorRevenue {
    // Primary revenue: winning query races
    query_race_winnings: QueryRaceRevenue {
        winner_percentage: 70,    // Winner gets 70% of query payment
        verifier_percentage: 15,  // Each verifier gets 15%

        // Performance bonuses
        sub_1ms_bonus: 1.5,      // 50% bonus for sub-1ms
        sub_5ms_bonus: 1.2,      // 20% bonus for sub-5ms
        accuracy_bonus: 1.1,     // 10% bonus for perfect accuracy
    },

    // Secondary revenue: providing specialized services
    specialization_premiums: SpecializationRevenue {
        zk_reconstruction: Premium(2.0),     // 2x for complex reconstructions
        cache_optimization: Premium(1.5),    // 1.5x for predictive caching
        edge_latency: Premium(1.3),          // 1.3x for geographic optimization
    },

    // Tertiary revenue: governance participation
    governance_rewards: GovernanceRevenue {
        voting_participation: BaseReward(10), // 10 INDEX tokens per vote
        proposal_creation: BaseReward(100),   // 100 INDEX tokens per accepted proposal
        network_improvement: VariableReward,  // Based on impact assessment
    },
}

impl NodeOperatorRevenue {
    pub fn calculate_daily_revenue(&self, node_performance: NodePerformance) -> Revenue {
        // Calculate based on actual performance metrics
        let queries_won = node_performance.queries_won_today;
        let average_query_payment = node_performance.average_winning_payment;
        let performance_bonuses = node_performance.performance_bonus_total;
        let specialization_premiums = node_performance.specialization_revenue;

        Revenue {
            base_query_revenue: queries_won * average_query_payment,
            performance_bonuses,
            specialization_premiums,
            governance_rewards: self.calculate_governance_rewards(&node_performance),
            total: self.sum_all_revenue_streams(),
        }
    }
}
```

### Cost Structure and Profitability
```rust
pub struct NodeOperatorCosts {
    // Infrastructure costs
    infrastructure: InfrastructureCosts {
        compute: MonthlyFixed(1000), // $1000/month for high-end server
        storage: MonthlyFixed(200),  // $200/month for NVMe storage
        bandwidth: VariableCost(0.1), // $0.10 per GB transferred
        monitoring: MonthlyFixed(50), // $50/month for monitoring tools
    },

    // Token staking requirements
    staking: StakingCosts {
        minimum_stake: TokenAmount(10000), // 10,000 INDEX tokens minimum
        opportunity_cost: StakingYield(5), // 5% APY opportunity cost
        slashing_risk: RiskPercentage(2),  // 2% max slashing per incident
    },

    // Operational costs
    operations: OperationalCosts {
        development: MonthlyFixed(500),  // $500/month for maintenance
        compliance: MonthlyFixed(100),   // $100/month for legal/compliance
        insurance: MonthlyFixed(100),    // $100/month for operational insurance
    },
}

impl NodeOperatorCosts {
    pub fn calculate_breakeven_performance(&self) -> BreakevenAnalysis {
        let monthly_fixed_costs = self.infrastructure.monthly_total()
            + self.operations.monthly_total()
            + self.staking.monthly_opportunity_cost();

        let queries_needed_per_day = monthly_fixed_costs / 30 / self.average_query_profit();

        BreakevenAnalysis {
            monthly_costs: monthly_fixed_costs,
            queries_needed_daily: queries_needed_per_day,
            minimum_win_rate: self.calculate_minimum_win_rate(),
            profit_at_scale: self.calculate_profit_projections(),
        }
    }
}
```

## Competitive Dynamics

### Node Competition Mechanics
```rust
pub struct CompetitionMechanics {
    // Racing creates direct competition
    query_racing: RacingCompetition {
        nodes_per_race: 3..=5,           // 3-5 nodes compete per query
        winner_takes_most: true,         // 70% to winner, 15% to verifiers
        performance_based: true,         // Faster response = higher probability of selection
    },

    // Reputation affects future selection
    reputation_system: ReputationSystem {
        performance_history: PerformanceWeight(40),  // 40% weight on past performance
        accuracy_history: AccuracyWeight(30),        // 30% weight on correctness
        uptime_history: UptimeWeight(20),            // 20% weight on availability
        stake_amount: StakeWeight(10),               // 10% weight on economic commitment
    },

    // Market entry and exit
    market_dynamics: MarketDynamics {
        entry_barriers: Low,             // Low barriers encourage competition
        exit_freedom: High,              // Easy to leave if unprofitable
        specialization_opportunities: High, // Multiple ways to differentiate
    },
}

impl CompetitionMechanics {
    pub fn select_racing_nodes(&self, query: Query) -> Vec<NodeId> {
        // Select nodes based on capability and reputation
        let capable_nodes = self.find_nodes_capable_of_query(&query);

        // Weight selection by reputation and recent performance
        let weighted_selection = capable_nodes.iter()
            .map(|node| (node.id, self.calculate_selection_weight(node)))
            .collect();

        // Randomly select based on weights (better nodes more likely)
        self.weighted_random_selection(weighted_selection, 5)
    }

    fn calculate_selection_weight(&self, node: &Node) -> f64 {
        let performance_score = node.recent_performance_score();
        let accuracy_score = node.recent_accuracy_score();
        let uptime_score = node.uptime_score();
        let stake_score = (node.stake_amount as f64).ln(); // Logarithmic stake benefit

        (performance_score * 0.4)
            + (accuracy_score * 0.3)
            + (uptime_score * 0.2)
            + (stake_score * 0.1)
    }
}
```

### Innovation Incentives
```rust
pub struct InnovationIncentives {
    // Operators compete on features and performance
    competitive_advantages: Vec<CompetitiveAdvantage> {
        // Speed optimization
        CompetitiveAdvantage::Speed {
            metric: AverageLatency,
            reward_mechanism: PerformanceBonuses,
            customer_benefit: FasterResponses,
        },

        // Accuracy optimization
        CompetitiveAdvantage::Accuracy {
            metric: CorrectResponseRate,
            reward_mechanism: AccuracyBonuses + ReputationBoost,
            customer_benefit: TrustedResults,
        },

        // Specialization
        CompetitiveAdvantage::Specialization {
            metric: SpecializedCapabilities,
            reward_mechanism: SpecializationPremiums,
            customer_benefit: AdvancedFeatures,
        },

        // Cost efficiency
        CompetitiveAdvantage::CostEfficiency {
            metric: OperationalCosts,
            reward_mechanism: HigherProfitMargins,
            customer_benefit: LowerPrices,
        },
    },

    // Network rewards innovation
    innovation_rewards: InnovationRewards {
        performance_improvements: GovernanceTokens,
        new_feature_development: CommunityGrants,
        open_source_contributions: ReputationBonus,
        research_and_development: ProtocolTreasuryFunding,
    },
}
```

## Economic Sustainability Model

### Revenue Distribution
```rust
pub struct RevenueDistribution {
    // Customer payments distributed across network participants
    customer_payments: CustomerPayments {
        node_operators: Percentage(50),      // 50% to node operators for service
        protocol_treasury: Percentage(20),   // 20% to protocol development
        data_providers: Percentage(20),      // 20% to Solana RPC providers
        governance_rewards: Percentage(10),  // 10% to STRM stakers
    },

    // Sustainable economics for all participants
    sustainability_metrics: SustainabilityMetrics {
        node_operator_roi: TargetRange(15..30), // 15-30% annual ROI target
        protocol_funding: SustainableDevelopment,
        customer_value: PositiveROI,            // Customers save money vs alternatives
    },
}

impl RevenueDistribution {
    pub fn ensure_sustainable_economics(&self) -> SustainabilityCheck {
        // Check that all participants benefit
        let node_profitability = self.calculate_node_profitability();
        let customer_savings = self.calculate_customer_savings_vs_alternatives();
        let protocol_funding = self.calculate_protocol_sustainability();

        SustainabilityCheck {
            nodes_profitable: node_profitability > 0.15, // 15% ROI minimum
            customers_benefit: customer_savings > 0.20,   // 20% savings vs centralized
            protocol_funded: protocol_funding.covers_development_costs(),

            overall_sustainable: node_profitability > 0.15
                && customer_savings > 0.20
                && protocol_funding.covers_development_costs(),
        }
    }
}
```

### Growth Economics
```rust
pub struct GrowthEconomics {
    // Network effects improve economics for everyone
    network_effects: NetworkEffects {
        more_nodes: BenefitsCustomers {
            lower_latency: GeographicDistribution,
            higher_reliability: Redundancy,
            better_specialization: DiverseCapabilities,
        },

        more_customers: BenefitsNodes {
            higher_revenue: MoreQueries,
            better_utilization: CapacityOptimization,
            specialization_opportunities: NicheMarkets,
        },

        virtuous_cycle: VirtuousCycle {
            better_service: AttractsMoreCustomers,
            more_revenue: AttractsMoreNodes,
            more_competition: ImproveService,
        },
    },

    // Scaling benefits
    scale_benefits: ScaleBenefits {
        cost_reduction: EconomiesOfScale,
        performance_improvement: NetworkOptimization,
        feature_acceleration: CompetitiveInnovation,
        market_expansion: GlobalReach,
    },
}
```

This economic model creates a **sustainable, competitive marketplace** where:
- **Customers** get better service at fair prices
- **Node operators** earn attractive returns for good service
- **The protocol** is self-funding through network fees
- **Innovation** is driven by competitive pressure

The market mechanisms ensure that good performance is rewarded, poor performance is penalized, and the network continuously improves through competition.