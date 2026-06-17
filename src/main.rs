use aerothesis::{Aerothesis, BoundaryType, OscillationType};
use rustfft::{num_complex::Complex, FftPlanner};
use textplots::{Chart, Plot, Shape};

fn custom_step(plugin: &mut Aerothesis, m: f32, r: f32, k: f32) -> f32 {
    let t = 1.0 / plugin.sample_rate;
    const EPS: f32 = 1e-5;
    const RHO: f32 = 1.2;

    // vf logic (Current flow velocity)
    let vf = {
        let a_fluid = (RHO * plugin.params.reed_length.value()) / t;
        let gap_prev = (2.0 - plugin.x_prev).clamp(EPS, 2.0);
        let b_prev = RHO / (4.0 * (gap_prev * gap_prev));
        let c_prev = plugin.v_breath - b_prev * (plugin.v_fluid_prev * plugin.v_fluid_prev);

        let gap_curr = (2.0 - plugin.x_prev).clamp(EPS, 2.0);
        let b_curr = RHO / (4.0 * (gap_curr * gap_curr));

        if gap_curr <= EPS {
            0.0
        } else {
            let disc = (a_fluid * a_fluid
                + 4.0 * b_curr * (a_fluid * plugin.v_fluid_prev + c_prev))
                .max(0.0);
            (-a_fluid + disc.sqrt()) / (2.0 * b_curr)
        }
    };

    // f logic (Force)
    let f = {
        let gap_curr = (2.0 - plugin.x_prev).clamp(EPS, 2.0);
        0.5 * RHO * (vf * vf) * gap_curr
    };

    // x logic (Displacement)
    let x_n = {
        let b0 = t * t;
        let b1 = 2.0 * t * t;
        let b2 = t * t;
        let a0 = 4.0 * m + 2.0 * r * t + k * t * t;
        let a1 = -8.0 * m + 2.0 * k * t * t;
        let a2 = 4.0 * m - 2.0 * r * t + k * t * t;
        ((b0 * f + b1 * plugin.f_prev + b2 * plugin.f_prev2
            - a1 * plugin.x_prev
            - a2 * plugin.x_prev2)
            / a0)
            .clamp(0.0, 2.0)
    };

    plugin.x_prev2 = plugin.x_prev;
    plugin.x_prev = x_n;
    plugin.f_prev2 = plugin.f_prev;
    plugin.f_prev = f;
    plugin.v_fluid_prev = vf;

    x_n
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = Aerothesis::default();
    let sample_rate = 44100.0;
    let seconds = 1.0;
    let num_samples = (sample_rate * seconds) as usize;
    let mut data = Vec::with_capacity(num_samples);
    let mut p_mouth_signal = Vec::with_capacity(num_samples);

    plugin.sample_rate = sample_rate;

    for i in 0..num_samples {
        plugin.v_breath = if i < 1000 {
            (i as f32 / 1000.0) * 40.0
        } else {
            40.0
        };

        let p_total = plugin.v_breath + plugin.p_minus;
        let (x_n, vf_n) = plugin.reed.step(
            p_total,
            plugin.params.oscillation_type.value(),
            plugin.sample_rate,
            &plugin.params,
            plugin.v_bite,
        );

        let gap = (2.0 - x_n).max(1e-5);
        let u = vf_n * gap;
        let p_mouth = plugin.p_minus + plugin.bore.z0 * u;
        let p_plus = p_mouth - plugin.p_minus;

        plugin.p_minus = plugin.bore.step(
            p_plus,
            1.0 / sample_rate,
            BoundaryType::NonlinearDissipative,
        );

        if i < (sample_rate * 0.05) as usize {
            data.push((i as f32, p_mouth));
        }
        p_mouth_signal.push(p_mouth);
    }

    Chart::new(180, 60, 0.0, data.len() as f32)
        .lineplot(&Shape::Lines(&data))
        .display();

    let fft_len = p_mouth_signal.len().next_power_of_two();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_len);

    let mut buffer: Vec<Complex<f32>> = p_mouth_signal
        .iter()
        .map(|&p| Complex { re: p, im: 0.0 })
        .collect();
    buffer.resize(fft_len, Complex { re: 0.0, im: 0.0 });

    fft.process(&mut buffer);

    let mut spectrum_data = Vec::with_capacity(fft_len / 2);
    for (i, complex) in buffer.iter().enumerate().take(fft_len / 2) {
        let freq = (i as f32 * sample_rate) / fft_len as f32;
        if freq > 1500.0 {
            break;
        }
        spectrum_data.push((freq, complex.norm()));
    }

    Chart::new(180, 60, 0.0, 1500.0)
        .lineplot(&Shape::Lines(&spectrum_data))
        .display();

    Ok(())
}
