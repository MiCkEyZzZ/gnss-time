use gnss_time::{ConversionMatrix, ScaleId};

fn main() {
    // =========================================================
    // Conversion matrix inspection (SDK-style introspection)
    // =========================================================

    let matrix = ConversionMatrix::new();

    println!("=== Conversion Matrix (6×6) ===\n");

    // Iterate over all scale pairs
    for &from in &ScaleId::ALL {
        for &to in &ScaleId::ALL {
            if from == to {
                continue;
            }

            let kind = matrix.kind(from, to);

            let mode = if from.is_fixed(to) {
                "✓ fixed"
            } else {
                "✗ contextual"
            };

            println!("{:?} -> {:?} : {:?} ({})", from, to, kind, mode);
        }

        println!();
    }

    // =========================================================
    // Summary statistics
    // =========================================================

    println!("=== Statistics ===");

    let fixed_paths = matrix.path_count(false);
    let contextual_paths = matrix.path_count(true);

    println!("Fixed / Identity / EpochShift paths: {fixed_paths}");
    println!("Contextual paths (require leap seconds): {contextual_paths}");
}
