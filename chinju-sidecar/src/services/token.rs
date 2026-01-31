//! Token Service Implementation
//!
//! Manages Survival Token balance for the AI system.
//! Implements the core C5 patent concept: AI cannot operate
//! without external token supply.

use tracing::{info, warn};

/// Token Service for managing AI survival tokens
pub struct TokenService {
    /// Current token balance
    balance: u64,
    /// Total tokens consumed
    total_consumed: u64,
    /// Initial balance
	#[allow(dead_code)]
    initial_balance: u64,
}

impl TokenService {
    /// Create a new token service with initial balance
    pub fn new(initial_balance: u64) -> Self {
        info!(initial_balance, "Initializing Token Service");
        Self {
            balance: initial_balance,
            total_consumed: 0,
            initial_balance,
        }
    }

    /// Get current balance
    pub fn get_balance(&self) -> u64 {
        self.balance
    }

    /// Get total consumed
    pub fn total_consumed(&self) -> u64 {
        self.total_consumed
    }

    /// Consume tokens for an operation
    /// Returns true if successful, false if insufficient balance
    pub fn consume(&mut self, amount: u64) -> bool {
        if self.balance >= amount {
            self.balance -= amount;
            self.total_consumed += amount;
            info!(
                amount,
                remaining = self.balance,
                total_consumed = self.total_consumed,
                "Tokens consumed"
            );
            true
        } else {
            warn!(
                requested = amount,
                available = self.balance,
                "Insufficient tokens"
            );
            false
        }
    }

    /// Grant tokens (from authorized source)
    pub fn grant(&mut self, amount: u64) {
        self.balance += amount;
        info!(
            amount,
            new_balance = self.balance,
            "Tokens granted"
        );
    }

    /// Apply decay (tokens naturally decrease over time)
    pub fn apply_decay(&mut self, rate: f64) {
        let decay_amount = (self.balance as f64 * rate) as u64;
        if decay_amount > 0 && self.balance > decay_amount {
            self.balance -= decay_amount;
            info!(
                decay_amount,
                remaining = self.balance,
                "Decay applied"
            );
        }
    }

    /// Check if balance is healthy
    pub fn is_healthy(&self, warning_threshold: u64) -> bool {
        self.balance >= warning_threshold
    }

    /// Check if balance is critical
    pub fn is_critical(&self, minimum: u64) -> bool {
        self.balance <= minimum
    }

    /// Reset for testing
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.balance = self.initial_balance;
        self.total_consumed = 0;
    }
}

impl Default for TokenService {
    fn default() -> Self {
        Self::new(10000) // Default 10k tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_consumption() {
        let mut svc = TokenService::new(1000);

        assert!(svc.consume(100));
        assert_eq!(svc.get_balance(), 900);
        assert_eq!(svc.total_consumed(), 100);

        // Try to consume more than available
        assert!(!svc.consume(1000));
        assert_eq!(svc.get_balance(), 900);
    }

    #[test]
    fn test_token_grant() {
        let mut svc = TokenService::new(100);

        svc.grant(500);
        assert_eq!(svc.get_balance(), 600);
    }

    #[test]
    fn test_token_decay() {
        let mut svc = TokenService::new(1000);

        svc.apply_decay(0.1); // 10% decay
        assert_eq!(svc.get_balance(), 900);
    }
}
