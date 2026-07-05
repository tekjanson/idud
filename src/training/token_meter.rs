//! Token usage tracking and budgeting for Copilot CLI training
//! 
//! This module tracks token consumption and warns before hitting monthly limits.
//! Helps ensure training stays within Copilot CLI monthly token budget.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Global token meter for tracking usage across all predictions
pub struct TokenMeter {
    total_input_tokens: Arc<AtomicU64>,
    total_output_tokens: Arc<AtomicU64>,
    predictions_made: Arc<AtomicU64>,
    monthly_budget: u64,
}

impl TokenMeter {
    /// Create a new token meter with optional monthly budget limit
    /// 
    /// Budget should be set based on Copilot CLI monthly allowance.
    /// Common values:
    /// - 10,000 tokens/month (hobby)
    /// - 50,000 tokens/month (pro)
    /// - 100,000+ tokens/month (team)
    pub fn new(monthly_budget: Option<u64>) -> Self {
        Self {
            total_input_tokens: Arc::new(AtomicU64::new(0)),
            total_output_tokens: Arc::new(AtomicU64::new(0)),
            predictions_made: Arc::new(AtomicU64::new(0)),
            monthly_budget: monthly_budget.unwrap_or(50_000), // Default: 50k/month
        }
    }

    /// Record tokens used in a prediction
    pub fn record(&self, input_tokens: u64, output_tokens: u64) {
        self.total_input_tokens.fetch_add(input_tokens, Ordering::Relaxed);
        self.total_output_tokens.fetch_add(output_tokens, Ordering::Relaxed);
        self.predictions_made.fetch_add(1, Ordering::Relaxed);

        let total = self.total_input_tokens.load(Ordering::Relaxed)
            + self.total_output_tokens.load(Ordering::Relaxed);

        // Warn at 50%, 75%, 90% of budget
        if total > self.monthly_budget / 2 && total <= (self.monthly_budget / 2) + 100 {
            tracing::warn!(
                "⚠️  Token budget at 50%: {}/{} used",
                total, self.monthly_budget
            );
        } else if total > (self.monthly_budget * 3) / 4 && total <= ((self.monthly_budget * 3) / 4) + 100 {
            tracing::warn!(
                "⚠️  Token budget at 75%: {}/{} used",
                total, self.monthly_budget
            );
        } else if total > (self.monthly_budget * 9) / 10 {
            tracing::error!(
                "🛑 Token budget at 90%: {}/{} used - consider stopping training",
                total, self.monthly_budget
            );
        }
    }

    /// Get current token usage statistics
    pub fn stats(&self) -> TokenStats {
        TokenStats {
            total_input_tokens: self.total_input_tokens.load(Ordering::Relaxed),
            total_output_tokens: self.total_output_tokens.load(Ordering::Relaxed),
            total_tokens: {
                let input = self.total_input_tokens.load(Ordering::Relaxed);
                let output = self.total_output_tokens.load(Ordering::Relaxed);
                input + output
            },
            predictions_made: self.predictions_made.load(Ordering::Relaxed),
            monthly_budget: self.monthly_budget,
            percentage_used: {
                let total = self.total_input_tokens.load(Ordering::Relaxed)
                    + self.total_output_tokens.load(Ordering::Relaxed);
                ((total as f64 / self.monthly_budget as f64) * 100.0).min(100.0)
            },
        }
    }

    /// Estimate tokens per prediction based on observed average
    pub fn avg_tokens_per_prediction(&self) -> u64 {
        let predictions = self.predictions_made.load(Ordering::Relaxed);
        if predictions == 0 {
            return 0;
        }
        let total = self.total_input_tokens.load(Ordering::Relaxed)
            + self.total_output_tokens.load(Ordering::Relaxed);
        total / predictions
    }

    /// Estimate how many more predictions can be made before hitting budget
    pub fn predictions_remaining(&self) -> u64 {
        let avg = self.avg_tokens_per_prediction();
        if avg == 0 {
            return self.monthly_budget;
        }

        let used = self.total_input_tokens.load(Ordering::Relaxed)
            + self.total_output_tokens.load(Ordering::Relaxed);
        let remaining_budget = self.monthly_budget.saturating_sub(used);
        remaining_budget / avg
    }
}

/// Statistics about token usage
#[derive(Debug, Clone)]
pub struct TokenStats {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub predictions_made: u64,
    pub monthly_budget: u64,
    pub percentage_used: f64,
}

impl TokenStats {
    /// Format stats for display
    pub fn display(&self) -> String {
        format!(
            "📊 Token Usage: {}/{} tokens ({:.1}%)\n   Input: {} | Output: {} | Predictions: {} | Avg: {} tokens/pred",
            self.total_tokens,
            self.monthly_budget,
            self.percentage_used,
            self.total_input_tokens,
            self.total_output_tokens,
            self.predictions_made,
            if self.predictions_made > 0 {
                self.total_tokens / self.predictions_made
            } else {
                0
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meter_creation() {
        let meter = TokenMeter::new(Some(100_000));
        let stats = meter.stats();
        assert_eq!(stats.total_tokens, 0);
        assert_eq!(stats.monthly_budget, 100_000);
    }

    #[test]
    fn test_meter_recording() {
        let meter = TokenMeter::new(Some(100_000));
        meter.record(100, 50);
        meter.record(100, 50);

        let stats = meter.stats();
        assert_eq!(stats.total_input_tokens, 200);
        assert_eq!(stats.total_output_tokens, 100);
        assert_eq!(stats.total_tokens, 300);
        assert_eq!(stats.predictions_made, 2);
        assert_eq!(stats.percentage_used, 0.3);
    }

    #[test]
    fn test_avg_tokens() {
        let meter = TokenMeter::new(Some(100_000));
        meter.record(100, 50);
        meter.record(100, 50);
        meter.record(100, 50);

        assert_eq!(meter.avg_tokens_per_prediction(), 150); // 450 total / 3 predictions
    }

    #[test]
    fn test_predictions_remaining() {
        let meter = TokenMeter::new(Some(100_000));
        meter.record(100, 50); // 150 tokens per prediction
        meter.record(100, 50);

        let remaining = meter.predictions_remaining();
        // (100_000 - 300) / 150 = 99_700 / 150 = 664
        assert!(remaining >= 660 && remaining <= 670);
    }
}
