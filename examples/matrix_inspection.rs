use gnss_time::{ConversionMatrix, ScaleId};

fn main() {
    // =========================================================
    // 1. Build conversion matrix (static graph of all paths)
    // =========================================================

    let matrix = ConversionMatrix::new();

    println!("=== Conversion Matrix (6×6) ===\n");

    // =========================================================
    // 2. Iterate over all scale pairs
    // =========================================================

    for &from in &ScaleId::ALL {
        for &to in &ScaleId::ALL {
            if from == to {
                continue;
            }

            // Conversion type (Identity / Fixed / Contextual / EpochShift)
            let kind = matrix.kind(from, to);

            // Runtime classification: requires leap seconds or not
            let classification = if from.is_fixed(to) {
                "✓ fixed"
            } else {
                "✗ contextual"
            };

            println!("{:?} -> {:?} : {:?} ({})", from, to, kind, classification);
        }

        println!();
    }

    // =========================================================
    // 3. Aggregate statistics (graph-level properties)
    // =========================================================

    println!("=== Statistics ===");

    println!(
        "Fixed + Identity + EpochShift paths: {}",
        matrix.path_count(false)
    );

    println!(
        "Contextual (leap-second dependent) paths: {}",
        matrix.path_count(true)
    );
}
