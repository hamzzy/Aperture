//! JSON output
//!
//! Exports profile data in JSON format for further analysis

use anyhow::{Context, Result};
use aperture_shared::types::profile::Profile;
use std::fs::File;
use std::io::BufWriter;
use tracing::info;

/// Generate JSON output from profile data
pub fn generate_json(profile: &Profile, output_path: &str) -> Result<()> {
    info!("Generating JSON output: {}", output_path);

    let file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path))?;

    let writer = BufWriter::new(file);

    serde_json::to_writer_pretty(writer, profile)
        .context("Failed to serialize profile to JSON")?;

    info!("JSON output written to {}", output_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_json() {
        let profile = Profile::new(0, 1000, 10_000_000);

        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("test.json");

        let result = generate_json(&profile, output_path.to_str().unwrap());
        assert!(result.is_ok());

        // Verify file was created
        assert!(output_path.exists());

        // Verify valid JSON
        let contents = std::fs::read_to_string(output_path).unwrap();
        let _parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
    }
}
