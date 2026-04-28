use gnss_time::{ConversionMatrix, ScaleId};

fn main() {
    let matrix = ConversionMatrix::new();

    println!("=== Матрица конверсий (6×6) ===\n");

    for &from in &ScaleId::ALL {
        for &to in &ScaleId::ALL {
            if from == to {
                continue;
            }

            let kind = matrix.kind(from, to);

            let fixed = if from.is_fixed(to) {
                "✓ fixed"
            } else {
                "✗ contextual"
            };

            println!("{:?} -> {:?} : {:?} ({})", from, to, kind, fixed);
        }

        println!();
    }

    println!("=== Статистика ===");
    println!(
        "Fixed + Identity + EpochShift путей: {}",
        matrix.path_count(false)
    );
    println!("Contextual путей: {}", matrix.path_count(true));
}
