//! IDL generation from behavioral patterns

use crate::{
    error::IDLResult,
    types::{
        IDLDefinition, InstructionDefinition, AccountDefinition, TypeDefinition,
        InstructionPattern, AccountStructure, FieldDefinition, FieldType,
        InstructionBehaviorPattern, AccountRequirement, EventDefinition,
        ErrorDefinition, ConstantDefinition, EventEmissionPattern, DataCorrelation,
        ErrorOccurrencePattern, ErrorTrigger, TriggerType, ErrorImpact, ConstantValue,
        CorrelationType, ErrorSeverity,
    },
};

use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};

/// IDL generator that creates IDL definitions from behavioral patterns
pub struct IDLGenerator {
    // Configuration for generation
    min_confidence_threshold: f64,
}

impl IDLGenerator {
    pub fn new() -> Self {
        Self {
            min_confidence_threshold: 0.7,
        }
    }

    /// Generate complete IDL definition from patterns
    pub async fn generate_idl_definition(
        &self,
        program_id: &Pubkey,
        instruction_patterns: &[InstructionPattern],
        account_structures: &[AccountStructure],
    ) -> IDLResult<IDLDefinition> {

        // Generate instructions from patterns
        let instructions = self.generate_instructions_from_patterns(instruction_patterns).await?;

        // Generate account definitions
        let accounts = self.generate_accounts_from_structures(account_structures).await?;

        // Generate type definitions (simplified)
        let types = self.generate_type_definitions(&accounts).await?;

        // Generate events from instruction patterns
        let events = self.generate_events_from_patterns(instruction_patterns).await?;

        // Generate errors from instruction patterns (analyzing failure cases)
        let errors = self.generate_errors_from_patterns(instruction_patterns).await?;

        // Generate constants from patterns
        let constants = self.generate_constants_from_patterns(instruction_patterns, account_structures).await?;

        Ok(IDLDefinition {
            version: "0.1.0".to_string(),
            name: format!("program_{}", program_id.to_string()[..8].to_lowercase()),
            instructions,
            accounts,
            types,
            events,
            errors,
            constants,
        })
    }

    /// Generate instruction definitions from patterns
    async fn generate_instructions_from_patterns(
        &self,
        patterns: &[InstructionPattern],
    ) -> IDLResult<Vec<InstructionDefinition>> {

        let mut instructions = Vec::new();

        for (i, pattern) in patterns.iter().enumerate() {
            let instruction_name = format!("instruction_{}", i);

            let instruction = InstructionDefinition {
                name: instruction_name,
                discriminator: pattern.instruction_data.clone(),
                accounts: self.generate_account_requirements(&pattern.account_pattern),
                args: vec![], // TODO: implement argument inference
                behavior_patterns: self.convert_to_behavior_pattern(pattern),
                frequency_stats: crate::types::InstructionFrequencyStats {
                    total_calls: 0, // TODO: calculate from patterns
                    unique_callers: 0,
                    calls_per_day_avg: 0.0,
                    peak_usage_times: vec![],
                    seasonal_patterns: None,
                },
            };

            instructions.push(instruction);
        }

        Ok(instructions)
    }

    /// Generate account definitions from structures
    async fn generate_accounts_from_structures(
        &self,
        structures: &[AccountStructure],
    ) -> IDLResult<Vec<AccountDefinition>> {

        let mut accounts = Vec::new();

        for structure in structures {
            let fields = self.generate_fields_from_layout(&structure.field_layout);

            let account = AccountDefinition {
                name: structure.account_type.clone(),
                fields,
                size: Some(structure.typical_size),
                discriminator: None, // TODO: infer discriminator
                access_patterns: crate::types::AccountAccessPattern {
                    read_frequency: 0.5, // TODO: calculate from patterns
                    write_frequency: 0.3,
                    concurrent_access_probability: 0.1,
                    typical_lifetime: Some(1000), // slots
                    size_distribution: crate::types::SizeDistribution {
                        min_size: structure.typical_size,
                        max_size: structure.typical_size,
                        avg_size: structure.typical_size as f64,
                        percentiles: HashMap::new(),
                    },
                },
            };

            accounts.push(account);
        }

        Ok(accounts)
    }

    /// Generate type definitions from accounts
    async fn generate_type_definitions(
        &self,
        accounts: &[AccountDefinition],
    ) -> IDLResult<Vec<TypeDefinition>> {

        let mut types = Vec::new();

        // Generate struct types for each account
        for account in accounts {
            let type_def = TypeDefinition {
                name: format!("{}Data", account.name),
                type_kind: crate::types::TypeKind::Struct,
                fields: account.fields.clone(),
                usage_patterns: crate::types::TypeUsagePattern {
                    usage_frequency: 1.0,
                    common_field_combinations: vec![],
                    serialization_patterns: crate::types::SerializationPattern::Borsh,
                },
            };

            types.push(type_def);
        }

        Ok(types)
    }

    /// Generate account requirements from patterns
    fn generate_account_requirements(
        &self,
        account_pattern: &[crate::types::AccountUsage],
    ) -> Vec<AccountRequirement> {

        account_pattern.iter().enumerate().map(|(i, usage)| {
            AccountRequirement {
                name: format!("account_{}", i),
                is_mut: matches!(usage.usage_type, crate::types::AccountUsageType::Write |
                                                   crate::types::AccountUsageType::Create |
                                                   crate::types::AccountUsageType::Delete |
                                                   crate::types::AccountUsageType::Resize),
                is_signer: false, // TODO: infer signer requirements
                is_optional: false,
                account_type: None, // TODO: infer account type
                constraints: crate::types::AccountConstraints {
                    owner: None,
                    size: usage.access_pattern.typical_size,
                    data_requirements: None,
                    balance_requirements: None,
                },
            }
        }).collect()
    }

    /// Convert instruction pattern to behavior pattern
    fn convert_to_behavior_pattern(&self, pattern: &InstructionPattern) -> InstructionBehaviorPattern {
        InstructionBehaviorPattern {
            typical_gas_usage: Some(pattern.gas_usage_pattern.avg_gas as u64),
            account_modifications: vec![], // TODO: implement
            typical_execution_time: Some(pattern.timing_pattern.avg_execution_time as u64),
            success_rate: pattern.success_rate,
            common_error_codes: vec![], // TODO: implement
        }
    }

    /// Generate field definitions from layout
    fn generate_fields_from_layout(
        &self,
        layout: &[crate::types::FieldLayout],
    ) -> Vec<FieldDefinition> {

        layout.iter().enumerate().map(|(i, field_layout)| {
            FieldDefinition {
                name: format!("field_{}", i),
                field_type: self.convert_inferred_type(&field_layout.field_type),
                optional: false, // TODO: infer optionality
                offset: Some(field_layout.offset),
                constraints: crate::types::FieldConstraints {
                    min_value: field_layout.constraints.min_value.clone(),
                    max_value: field_layout.constraints.max_value.clone(),
                    allowed_values: None,
                    pattern: field_layout.constraints.pattern.clone(),
                },
            }
        }).collect()
    }

    /// Convert inferred field type to IDL field type
    fn convert_inferred_type(&self, inferred_type: &crate::types::InferredFieldType) -> FieldType {
        match inferred_type {
            crate::types::InferredFieldType::Integer { bits, signed } => {
                match (bits, signed) {
                    (8, false) => FieldType::U8,
                    (16, false) => FieldType::U16,
                    (32, false) => FieldType::U32,
                    (64, false) => FieldType::U64,
                    (8, true) => FieldType::I8,
                    (16, true) => FieldType::I16,
                    (32, true) => FieldType::I32,
                    (64, true) => FieldType::I64,
                    _ => FieldType::U64, // Default fallback
                }
            },
            crate::types::InferredFieldType::FloatingPoint { bits } => {
                match bits {
                    32 => FieldType::F32,
                    64 => FieldType::F64,
                    _ => FieldType::F64, // Default fallback
                }
            },
            crate::types::InferredFieldType::Boolean => FieldType::Bool,
            crate::types::InferredFieldType::PublicKey => FieldType::Pubkey,
            crate::types::InferredFieldType::String { .. } => FieldType::String,
            crate::types::InferredFieldType::Bytes { .. } => FieldType::Bytes,
            crate::types::InferredFieldType::Array { element_type, length } => {
                FieldType::Array(
                    Box::new(self.convert_inferred_type(element_type)),
                    *length
                )
            },
            crate::types::InferredFieldType::Option { inner_type } => {
                FieldType::Option(Box::new(self.convert_inferred_type(inner_type)))
            },
            crate::types::InferredFieldType::Enum { variants: _ } => {
                FieldType::U8 // Simplified enum representation
            },
            crate::types::InferredFieldType::Struct { fields: _ } => {
                FieldType::Defined("CustomStruct".to_string()) // Simplified struct reference
            },
        }
    }

    /// Generate event definitions from instruction patterns
    /// Events are inferred from instruction success patterns and state changes
    async fn generate_events_from_patterns(
        &self,
        patterns: &[InstructionPattern],
    ) -> IDLResult<Vec<EventDefinition>> {
        let mut events = Vec::new();
        let mut seen_discriminators: HashSet<Vec<u8>> = HashSet::new();

        for (i, pattern) in patterns.iter().enumerate() {
            // Skip patterns with low success rate (likely not event-producing)
            if pattern.success_rate < self.min_confidence_threshold {
                continue;
            }

            // Create a unique discriminator for this event
            let discriminator = if pattern.instruction_data.len() >= 8 {
                pattern.instruction_data[..8].to_vec()
            } else {
                let mut disc = vec![0u8; 8];
                disc[..pattern.instruction_data.len()].copy_from_slice(&pattern.instruction_data);
                disc
            };

            // Skip duplicate discriminators
            if seen_discriminators.contains(&discriminator) {
                continue;
            }
            seen_discriminators.insert(discriminator.clone());

            // Generate event fields based on account modifications
            let mut fields = Vec::new();

            // Add common event fields
            fields.push(FieldDefinition {
                name: "instruction_index".to_string(),
                field_type: FieldType::U8,
                optional: false,
                offset: Some(0),
                constraints: crate::types::FieldConstraints {
                    min_value: None,
                    max_value: None,
                    allowed_values: None,
                    pattern: None,
                },
            });

            // Add fields based on account pattern
            for (j, account_usage) in pattern.account_pattern.iter().enumerate() {
                if matches!(account_usage.usage_type,
                    crate::types::AccountUsageType::Write |
                    crate::types::AccountUsageType::Create) {
                    fields.push(FieldDefinition {
                        name: format!("modified_account_{}", j),
                        field_type: FieldType::Pubkey,
                        optional: false,
                        offset: Some(1 + j * 32),
                        constraints: crate::types::FieldConstraints {
                            min_value: None,
                            max_value: None,
                            allowed_values: None,
                            pattern: None,
                        },
                    });
                }
            }

            let event = EventDefinition {
                name: format!("Instruction{}Event", i),
                fields,
                discriminator,
                emission_patterns: EventEmissionPattern {
                    emission_frequency: pattern.success_rate,
                    typical_triggers: vec![format!("instruction_{}_execution", i)],
                    data_correlation: vec![
                        DataCorrelation {
                            field_name: "instruction_index".to_string(),
                            correlation_strength: 1.0,
                            correlation_type: CorrelationType::Positive,
                        }
                    ],
                },
            };

            events.push(event);
        }

        Ok(events)
    }

    /// Generate error definitions from instruction patterns
    /// Errors are inferred from failed instruction patterns
    async fn generate_errors_from_patterns(
        &self,
        patterns: &[InstructionPattern],
    ) -> IDLResult<Vec<ErrorDefinition>> {
        let mut errors = Vec::new();
        let mut error_code = 6000u32; // Start from custom error range

        for (i, pattern) in patterns.iter().enumerate() {
            // Instructions with low success rate indicate common errors
            let failure_rate = 1.0 - pattern.success_rate;

            if failure_rate > 0.01 {
                // Infer error based on common failure scenarios
                let error_triggers = self.infer_error_triggers(pattern);

                // Insufficient funds error
                if pattern.gas_usage_pattern.avg_gas > 0.0 {
                    errors.push(ErrorDefinition {
                        name: format!("Instruction{}InsufficientFunds", i),
                        code: error_code,
                        message: format!("Insufficient funds for instruction {}", i),
                        occurrence_patterns: ErrorOccurrencePattern {
                            frequency: failure_rate * 0.3, // Estimate 30% of failures are fund-related
                            common_triggers: vec![
                                ErrorTrigger {
                                    trigger_type: TriggerType::InsufficientFunds,
                                    description: "Account balance too low".to_string(),
                                    probability: 0.3,
                                },
                            ],
                            user_impact: ErrorImpact {
                                severity: ErrorSeverity::Medium,
                                recoverability: true,
                                typical_resolution: Some("Add more funds to account".to_string()),
                            },
                        },
                    });
                    error_code += 1;
                }

                // Invalid account state error
                if !pattern.account_pattern.is_empty() {
                    errors.push(ErrorDefinition {
                        name: format!("Instruction{}InvalidAccountState", i),
                        code: error_code,
                        message: format!("Invalid account state for instruction {}", i),
                        occurrence_patterns: ErrorOccurrencePattern {
                            frequency: failure_rate * 0.4,
                            common_triggers: error_triggers,
                            user_impact: ErrorImpact {
                                severity: ErrorSeverity::High,
                                recoverability: false,
                                typical_resolution: Some("Check account state before calling".to_string()),
                            },
                        },
                    });
                    error_code += 1;
                }

                // Access denied error (if signer patterns detected)
                errors.push(ErrorDefinition {
                    name: format!("Instruction{}AccessDenied", i),
                    code: error_code,
                    message: format!("Access denied for instruction {}", i),
                    occurrence_patterns: ErrorOccurrencePattern {
                        frequency: failure_rate * 0.2,
                        common_triggers: vec![
                            ErrorTrigger {
                                trigger_type: TriggerType::PermissionDenied,
                                description: "Missing required signer".to_string(),
                                probability: 0.2,
                            },
                        ],
                        user_impact: ErrorImpact {
                            severity: ErrorSeverity::Medium,
                            recoverability: true,
                            typical_resolution: Some("Ensure correct signer is provided".to_string()),
                        },
                    },
                });
                error_code += 1;
            }
        }

        Ok(errors)
    }

    /// Infer error triggers from instruction pattern
    fn infer_error_triggers(&self, pattern: &InstructionPattern) -> Vec<ErrorTrigger> {
        let mut triggers = Vec::new();

        // Check for write operations that might fail
        for usage in &pattern.account_pattern {
            match usage.usage_type {
                crate::types::AccountUsageType::Write => {
                    triggers.push(ErrorTrigger {
                        trigger_type: TriggerType::AccountConstraintViolation,
                        description: "Account not writable or locked".to_string(),
                        probability: 0.2,
                    });
                }
                crate::types::AccountUsageType::Create => {
                    triggers.push(ErrorTrigger {
                        trigger_type: TriggerType::AccountConstraintViolation,
                        description: "Account already exists".to_string(),
                        probability: 0.15,
                    });
                }
                crate::types::AccountUsageType::Delete => {
                    triggers.push(ErrorTrigger {
                        trigger_type: TriggerType::AccountConstraintViolation,
                        description: "Cannot delete non-empty account".to_string(),
                        probability: 0.1,
                    });
                }
                _ => {}
            }
        }

        if triggers.is_empty() {
            triggers.push(ErrorTrigger {
                trigger_type: TriggerType::Custom("Unknown".to_string()),
                description: "Unknown error condition".to_string(),
                probability: 0.5,
            });
        }

        triggers
    }

    /// Generate constant definitions from patterns
    /// Constants are inferred from repeated values in instruction data
    async fn generate_constants_from_patterns(
        &self,
        instruction_patterns: &[InstructionPattern],
        account_structures: &[AccountStructure],
    ) -> IDLResult<Vec<ConstantDefinition>> {
        let mut constants = Vec::new();
        let mut seen_values: HashSet<Vec<u8>> = HashSet::new();

        // Extract discriminators as constants
        for (i, pattern) in instruction_patterns.iter().enumerate() {
            if pattern.instruction_data.len() >= 8 {
                let disc = pattern.instruction_data[..8].to_vec();
                if !seen_values.contains(&disc) {
                    seen_values.insert(disc.clone());

                    // Convert to u64 for discriminator
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&disc);
                    let disc_value = u64::from_le_bytes(bytes);

                    constants.push(ConstantDefinition {
                        name: format!("INSTRUCTION_{}_DISCRIMINATOR", i),
                        value: ConstantValue::U64(disc_value),
                        usage_context: vec![
                            format!("instruction_{}", i),
                            "discriminator".to_string(),
                        ],
                    });
                }
            }
        }

        // Extract common account sizes as constants
        for structure in account_structures {
            constants.push(ConstantDefinition {
                name: format!("{}_SIZE", structure.account_type.to_uppercase()),
                value: ConstantValue::U64(structure.typical_size as u64),
                usage_context: vec![
                    structure.account_type.clone(),
                    "account_size".to_string(),
                ],
            });
        }

        // Add standard Solana constants that programs commonly use
        constants.push(ConstantDefinition {
            name: "PUBKEY_SIZE".to_string(),
            value: ConstantValue::U64(32),
            usage_context: vec!["common".to_string(), "size".to_string()],
        });

        constants.push(ConstantDefinition {
            name: "SIGNATURE_SIZE".to_string(),
            value: ConstantValue::U64(64),
            usage_context: vec!["common".to_string(), "size".to_string()],
        });

        Ok(constants)
    }
}

impl Default for IDLGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_idl_generator_creation() {
        let generator = IDLGenerator::new();
        assert_eq!(generator.min_confidence_threshold, 0.7);
    }

    #[tokio::test]
    async fn test_empty_patterns_generation() {
        let generator = IDLGenerator::new();
        let program_id = Pubkey::new_unique();

        let result = generator.generate_idl_definition(
            &program_id,
            &[],
            &[]
        ).await;

        assert!(result.is_ok());
        let idl = result.unwrap();
        assert_eq!(idl.instructions.len(), 0);
        assert_eq!(idl.accounts.len(), 0);
        assert!(idl.name.starts_with("program_"));
    }

    #[test]
    fn test_inferred_type_conversion() {
        let generator = IDLGenerator::new();

        // Test integer conversion
        let u32_type = crate::types::InferredFieldType::Integer { bits: 32, signed: false };
        assert!(matches!(generator.convert_inferred_type(&u32_type), FieldType::U32));

        let i64_type = crate::types::InferredFieldType::Integer { bits: 64, signed: true };
        assert!(matches!(generator.convert_inferred_type(&i64_type), FieldType::I64));

        // Test boolean conversion
        let bool_type = crate::types::InferredFieldType::Boolean;
        assert!(matches!(generator.convert_inferred_type(&bool_type), FieldType::Bool));

        // Test pubkey conversion
        let pubkey_type = crate::types::InferredFieldType::PublicKey;
        assert!(matches!(generator.convert_inferred_type(&pubkey_type), FieldType::Pubkey));
    }
}