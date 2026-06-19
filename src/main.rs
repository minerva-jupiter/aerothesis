use aerothesis::Aerothesis;
use nih_plug::prelude::util;
use rustfft::{num_complex::Complex, FftPlanner};
use textplots::{Chart, Plot, Shape};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = Aerothesis::default();
    let sample_rate = 44100.0;
    let seconds = 0.5;
    let num_samples = (sample_rate * seconds) as usize;

    plugin.sample_rate = sample_rate;
    plugin.note_frequency = util::midi_note_to_freq(48); // C3 (u8)

    // For simulation in main.rs, we use the default parameters from AerothesisParams::default()
    // because nih-plug parameters are designed to be managed by a host and don't have
    // simple setter methods for plain values without a ParamSetter context.
    // Default resonance: OpenPipe, Decay: 0.9

    let mut data = Vec::with_capacity(num_samples);
    let mut signal = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        // Simple attack envelope for breath pressure
        plugin.v_breath = if i < 2000 {
            (i as f32 / 2000.0) * 100.0
        } else {
            100.0
        };

        let sample = plugin.resonance();

        // Collect first 50ms for waveform plot
        if i < (sample_rate * 0.05) as usize {
            data.push((i as f32, sample));
        }
        signal.push(sample);
    }

    println!("--- Waveform (first 50ms) ---");
    Chart::new(180, 60, 0.0, data.len() as f32)
        .lineplot(&Shape::Lines(&data))
        .display();

    let fft_len = signal.len().next_power_of_two();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_len);

    let mut buffer: Vec<Complex<f32>> =
        signal.iter().map(|&s| Complex { re: s, im: 0.0 }).collect();
    buffer.resize(fft_len, Complex { re: 0.0, im: 0.0 });

    fft.process(&mut buffer);

    let mut spectrum_data = Vec::with_capacity(fft_len / 2);
    for (i, complex) in buffer.iter().enumerate().take(fft_len / 2) {
        let freq = (i as f32 * sample_rate) / fft_len as f32;
        if freq > 2000.0 {
            break;
        }
        spectrum_data.push((freq, complex.norm()));
    }

    println!("\n--- Spectrum (0 - 2000Hz) ---");
    Chart::new(180, 60, 0.0, 2000.0)
        .lineplot(&Shape::Lines(&spectrum_data))
        .display();

    Ok(())
}
