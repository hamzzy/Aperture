//! Flamegraph generation
//!
//! Generates SVG flamegraphs from profile data using the inferno library

use anyhow::{Context, Result};
use aperture_shared::types::profile::Profile;
use inferno::flamegraph;
use std::fs::File;
use std::io::{BufWriter, Write};
use tracing::info;

/// Generate a flamegraph from profile data
pub fn generate_flamegraph(profile: &Profile, output_path: &str) -> Result<()> {
    info!("Generating flamegraph: {}", output_path);

    // Convert Profile to inferno's folded stack format
    let mut folded_lines = Vec::new();
    for (stack, count) in &profile.samples {
        // Build folded stack line: func1;func2;func3 count
        let mut frame_names = Vec::new();

        // Reverse frames (flamegraphs show bottom-up)
        for frame in stack.frames.iter().rev() {
            let name = if let Some(func) = &frame.function {
                func.clone()
            } else {
                format!("0x{:x}", frame.ip)
            };
            frame_names.push(name);
        }

        if !frame_names.is_empty() {
            let folded_line = format!("{} {}", frame_names.join(";"), count);
            folded_lines.push(folded_line);
        }
    }

    if folded_lines.is_empty() {
        return Err(anyhow::anyhow!("No samples to generate flamegraph"));
    }

    info!("Generated {} folded stack lines", folded_lines.len());

    // Create output file
    let output_file = File::create(output_path)
        .with_context(|| format!("Failed to create flamegraph file: {}", output_path))?;

    let writer = BufWriter::new(output_file);

    // Configure flamegraph options
    let mut options = flamegraph::Options::default();
    options.title = "CPU Profile Flamegraph".to_string();
    options.count_name = "samples".to_string();
    options.flame_chart = false; // Use regular flamegraph (aggregated)

    // Convert folded lines to reader
    let folded_data = folded_lines.join("\n");
    let reader = std::io::Cursor::new(folded_data.as_bytes());

    // Generate flamegraph SVG
    flamegraph::from_reader(&mut options, reader, writer)
        .context("Failed to generate flamegraph SVG")?;

    info!("Flamegraph generated successfully: {}", output_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aperture_shared::types::profile::{Frame, Stack};
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_generate_flamegraph() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.svg");

        // Create a simple profile
        let mut profile = Profile::new(0, 1000, 10_000_000);

        // Add some samples
        let stack1 = Stack {
            frames: vec![
                Frame {
                    ip: 0x400000,
                    function: Some("main".to_string()),
                    file: None,
                    line: None,
                    module: None,
                },
                Frame {
                    ip: 0x400100,
                    function: Some("process".to_string()),
                    file: None,
                    line: None,
                    module: None,
                },
            ],
        };

        profile.add_sample(stack1.clone());
        profile.add_sample(stack1);

        let result = generate_flamegraph(&profile, output_path.to_str().unwrap());
        assert!(result.is_ok());
        assert!(output_path.exists());
    }
}
