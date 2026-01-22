//! Configuration types for the profiling agent

use std::time::Duration;

/// Agent configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Target process ID (None = profile all processes)
    pub target_pid: Option<i32>,

    /// Sampling rate in Hz
    pub sample_rate_hz: u64,

    /// Profiling duration
    pub duration: Duration,

    /// Output path for flamegraph
    pub output_path: String,

    /// Optional JSON output path
    pub json_output: Option<String>,
}

impl Config {
    /// Calculate the sampling period in nanoseconds
    pub fn sample_period_ns(&self) -> u64 {
        if self.sample_rate_hz == 0 {
            return 0;
        }
        1_000_000_000 / self.sample_rate_hz
    }

    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.sample_rate_hz == 0 {
            anyhow::bail!("Sample rate must be greater than 0");
        }

        if self.sample_rate_hz > 10000 {
            anyhow::bail!("Sample rate too high (max 10000 Hz)");
        }

        if self.duration.as_secs() == 0 {
            anyhow::bail!("Duration must be greater than 0");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_period_calculation() {
        let config = Config {
            target_pid: None,
            sample_rate_hz: 100,
            duration: Duration::from_secs(10),
            output_path: "test.svg".to_string(),
            json_output: None,
        };

        assert_eq!(config.sample_period_ns(), 10_000_000);
    }

    #[test]
    fn test_config_validation() {
        let valid = Config {
            target_pid: None,
            sample_rate_hz: 99,
            duration: Duration::from_secs(30),
            output_path: "test.svg".to_string(),
            json_output: None,
        };

        assert!(valid.validate().is_ok());

        let invalid = Config {
            target_pid: None,
            sample_rate_hz: 0,
            duration: Duration::from_secs(30),
            output_path: "test.svg".to_string(),
            json_output: None,
        };

        assert!(invalid.validate().is_err());
    }
}
