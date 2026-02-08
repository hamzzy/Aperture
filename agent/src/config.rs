//! Configuration types for the profiling agent

use std::path::PathBuf;
use std::time::Duration;

/// Profiling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileMode {
    Cpu,
    Lock,
    Syscall,
    All,
}

impl std::str::FromStr for ProfileMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cpu" => Ok(ProfileMode::Cpu),
            "lock" => Ok(ProfileMode::Lock),
            "syscall" => Ok(ProfileMode::Syscall),
            "all" => Ok(ProfileMode::All),
            _ => anyhow::bail!("Invalid profile mode: {}", s),
        }
    }
}

/// Agent configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Profiling mode
    pub mode: ProfileMode,

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

    /// Optional WASM filter path
    pub filter_path: Option<PathBuf>,

    /// Optional aggregator gRPC URL (e.g. http://127.0.0.1:50051) to push profile data
    pub aggregator_url: Option<String>,

    /// Push interval in seconds when streaming to aggregator (None = library default, e.g. 5s).
    /// Set via APERTURE_LOW_OVERHEAD=1 for lower CPU/network overhead (e.g. 10s).
    pub push_interval_secs: Option<u64>,
}

impl Config {
    /// Calculate the sampling period in nanoseconds
    pub fn sample_period_ns(&self) -> u64 {
        if self.sample_rate_hz == 0 {
            return 0;
        }
        1_000_000_000 / self.sample_rate_hz
    }

    /// Push interval for aggregator streaming (default 5s). Use for CPU/network overhead tuning.
    pub fn push_interval(&self) -> Duration {
        self.push_interval_secs
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(5))
    }

    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        // Sample rate only matters for CPU profiling
        if matches!(self.mode, ProfileMode::Cpu | ProfileMode::All) {
            if self.sample_rate_hz == 0 {
                anyhow::bail!("Sample rate must be greater than 0 for CPU profiling");
            }

            if self.sample_rate_hz > 10000 {
                anyhow::bail!("Sample rate too high (max 10000 Hz)");
            }
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
            mode: ProfileMode::Cpu,
            target_pid: None,
            sample_rate_hz: 100,
            duration: Duration::from_secs(10),
            output_path: "test.svg".to_string(),
            json_output: None,
            filter_path: None,
            aggregator_url: None,
            push_interval_secs: None,
        };

        assert_eq!(config.sample_period_ns(), 10_000_000);
    }

    #[test]
    fn test_config_validation() {
        let valid = Config {
            mode: ProfileMode::Cpu,
            target_pid: None,
            sample_rate_hz: 99,
            duration: Duration::from_secs(30),
            output_path: "test.svg".to_string(),
            json_output: None,
            filter_path: None,
            aggregator_url: None,
            push_interval_secs: None,
        };

        assert!(valid.validate().is_ok());

        let invalid = Config {
            mode: ProfileMode::Cpu,
            target_pid: None,
            sample_rate_hz: 0,
            duration: Duration::from_secs(30),
            output_path: "test.svg".to_string(),
            json_output: None,
            filter_path: None,
            aggregator_url: None,
            push_interval_secs: None,
        };

        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_validation_rate_too_high() {
        let config = Config {
            mode: ProfileMode::Cpu,
            target_pid: None,
            sample_rate_hz: 10001,
            duration: Duration::from_secs(5),
            output_path: "test.svg".to_string(),
            json_output: None,
            filter_path: None,
            aggregator_url: None,
            push_interval_secs: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_max_rate_ok() {
        let config = Config {
            mode: ProfileMode::Cpu,
            target_pid: None,
            sample_rate_hz: 10000,
            duration: Duration::from_secs(5),
            output_path: "test.svg".to_string(),
            json_output: None,
            filter_path: None,
            aggregator_url: None,
            push_interval_secs: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_zero_duration() {
        let config = Config {
            mode: ProfileMode::Cpu,
            target_pid: None,
            sample_rate_hz: 99,
            duration: Duration::from_secs(0),
            output_path: "test.svg".to_string(),
            json_output: None,
            filter_path: None,
            aggregator_url: None,
            push_interval_secs: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sample_period_zero_rate() {
        let config = Config {
            mode: ProfileMode::Cpu,
            target_pid: None,
            sample_rate_hz: 0,
            duration: Duration::from_secs(1),
            output_path: "test.svg".to_string(),
            json_output: None,
            filter_path: None,
            aggregator_url: None,
            push_interval_secs: None,
        };
        assert_eq!(config.sample_period_ns(), 0);
    }

    #[test]
    fn test_push_interval_default_and_override() {
        let default_config = Config {
            mode: ProfileMode::Cpu,
            target_pid: None,
            sample_rate_hz: 99,
            duration: Duration::from_secs(10),
            output_path: "out.svg".to_string(),
            json_output: None,
            filter_path: None,
            aggregator_url: None,
            push_interval_secs: None,
        };
        assert_eq!(default_config.push_interval(), Duration::from_secs(5));

        let low_overhead_config = Config {
            push_interval_secs: Some(10),
            ..default_config.clone()
        };
        assert_eq!(low_overhead_config.push_interval(), Duration::from_secs(10));
    }
}
