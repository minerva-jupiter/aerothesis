use aerothesis::Aerothesis;
use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = Aerothesis::default();

    // Set some test values
    plugin.v_breath = 0.8;

    let output_dir = Path::new("output");
    create_dir_all(&output_dir)?;

    let mut w = BufWriter::new(File::create(&output_dir.join("results.csv"))?);

    println!("Starting simulation...");

    for i in 0..10000 {
        let x = plugin.step();
        let vf = plugin.v_fluid_prev; // Updated in step()
        writeln!(w, "{}, {}, {}", i, x, vf)?;
    }

    Ok(())
}
