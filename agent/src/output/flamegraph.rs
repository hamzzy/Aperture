//! Flamegraph generation
//!
//! Generates SVG flamegraphs from profile data using the inferno library

use anyhow::{Context, Result};
use shared::types::profile::Profile;
use std::fs::File;
use std::io::BufWriter;
use tracing::info;

/// Generate a flamegraph from profile data
pub fn generate_flamegraph(profile: &Profile, output_path: &str) -> Result<()> {
    info!("Generating flamegraph: {}", output_path);

    // TODO Phase 1: Implement flamegraph generation
    // 1. Convert Profile to inferno's format (folded stacks)
    // 2. Use inferno::flamegraph to generate SVG
    // 3. Write to output file

    // Create output file
    let file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path))?;

    let mut writer = BufWriter::new(file);

    // Placeholder: write a simple SVG with a message
    use std::io::Write;
    writeln!(
        writer,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="200">"#
    )?;
    writeln!(writer, r#"  <text x="50" y="100" font-size="20">"#)?;
    writeln!(
        writer,
        "    Flamegraph generation not yet implemented"
    )?;
    writeln!(writer, "    Profile: {} samples", profile.total_samples)?;
    writeln!(writer, "  </text>")?;
    writeln!(writer, "</svg>")?;

    info!("Flamegraph written to {}", output_path);

    Ok(())
}

/// Convert profile to folded stack format (for inferno)
fn profile_to_folded(_profile: &Profile) -> Vec<String> {
    // TODO Phase 1: Implement folded stack format conversion
    // Format: "func1;func2;func3 count"
    // Example: "main;process;handle_request 42"

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_flamegraph() {
        let profile = Profile::new(0, 1000, 10_000_000);

        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("test.svg");

        let result = generate_flamegraph(&profile, output_path.to_str().unwrap());
        assert!(result.is_ok());

        // Verify file was created
        assert!(output_path.exists());
    }
}
