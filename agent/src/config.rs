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
        };
        assert_eq!(config.sample_period_ns(), 0);
    }
}
