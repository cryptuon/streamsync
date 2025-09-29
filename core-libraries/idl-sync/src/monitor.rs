//! Real-time monitoring for program changes

use crate::{
    error::IDLResult,
};

use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;

/// Real-time monitor for program changes
pub struct RealTimeMonitor {
    // Currently monitored programs
    monitored_programs: HashSet<Pubkey>,
}

impl RealTimeMonitor {
    pub fn new() -> Self {
        Self {
            monitored_programs: HashSet::new(),
        }
    }

    /// Start monitoring a program for changes
    pub async fn start_monitoring_program(&mut self, program_id: &Pubkey) -> IDLResult<()> {
        // In production, this would:
        // 1. Subscribe to program account changes
        // 2. Set up transaction stream filtering
        // 3. Initialize change detection

        self.monitored_programs.insert(*program_id);
        Ok(())
    }

    /// Stop monitoring a program
    pub async fn stop_monitoring_program(&mut self, program_id: &Pubkey) -> IDLResult<()> {
        self.monitored_programs.remove(program_id);
        Ok(())
    }

    /// Check if program is being monitored
    pub fn is_monitoring(&self, program_id: &Pubkey) -> bool {
        self.monitored_programs.contains(program_id)
    }

    /// Get list of monitored programs
    pub fn get_monitored_programs(&self) -> Vec<Pubkey> {
        self.monitored_programs.iter().cloned().collect()
    }
}

impl Default for RealTimeMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_creation() {
        let monitor = RealTimeMonitor::new();
        assert_eq!(monitor.monitored_programs.len(), 0);
    }

    #[tokio::test]
    async fn test_start_stop_monitoring() {
        let mut monitor = RealTimeMonitor::new();
        let program_id = Pubkey::new_unique();

        assert!(!monitor.is_monitoring(&program_id));

        monitor.start_monitoring_program(&program_id).await.unwrap();
        assert!(monitor.is_monitoring(&program_id));

        monitor.stop_monitoring_program(&program_id).await.unwrap();
        assert!(!monitor.is_monitoring(&program_id));
    }
}