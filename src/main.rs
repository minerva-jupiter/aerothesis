use aerothesis::Aerothesis;
use nih_plug::prelude::util;
use rustfft::{num_complex::Complex, FftPlanner};
use textplots::{Chart, Plot, Shape};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = Aerothesis::default();
    let sample_rate = 44100.0;
    let seconds = 1.0;
    let num_samples = (sample_rate * seconds) as usize;

    plugin.sample_rate = sample_rate;
    plugin.note_frequency = util::midi_note_to_freq(60); // C4 (u8)
                                                         // plugin.note_frequency = util::midi_note_to_freq(54); // F#3 (u8)

    // For simulation in main.rs, we use the default parameters from AerothesisParams::default()
    // because nih-plug parameters are designed to be managed by a host and don't have
    // simple setter methods for plain values without a ParamSetter context.
    // Default resonance: OpenPipe, Decay: 0.9

    let mut displacements = Vec::with_capacity(num_samples);
    let mut osc = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        // let x_current = self.resonance() - self.avg_x_history();

        // for sample in channel_samples {
        //     *sample = (x_current * gain).clamp(-1.0, 1.0);
        // }

        // Simple attack envelope for breath pressure
        plugin.v_breath = if i < 2000 {
            (i as f32 / 2000.0) * 100.0
        } else {
            100.0
        };

        let sample = plugin.displacement() - plugin.avg_x_history();
        osc.push(plugin.osc_x());
        displacements.push(sample);
    }

    let len = (sample_rate * 0.05) as usize;

    let data: Vec<(f32, f32)> = (0..len)
        .map(|i| (i as f32, displacements[i + (sample_rate * 0.1) as usize]))
        .collect();

    println!("--- Waveform ---");
    Chart::new(360, 60, 0.0, len as f32)
        .lineplot(&Shape::Lines(&data))
        .display();

    let fft_len = displacements.len().next_power_of_two();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_len);

    let mut buffer: Vec<Complex<f32>> = displacements
        .iter()
        .map(|&s| Complex { re: s, im: 0.0 })
        .collect();
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
    if let Some(max) = spectrum_data
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.1.partial_cmp(&b.1).unwrap())
    {
        println!("max freq is {} Hz", max.0);
    }

    println!("\n--- Spectrum (0 - 2000Hz) ---");
    Chart::new(180, 60, 0.0, 2000.0)
        .lineplot(&Shape::Lines(&spectrum_data))
        .display();

    Ok(())
}
