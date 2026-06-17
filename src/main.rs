use aerothesis::Aerothesis;
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
    let seconds = 0.02;
    let num_samples = (sample_rate * seconds) as usize;
    let mut data = Vec::with_capacity(num_samples);

    // Balance for amplitude: m=0.0005, k=2000 (fn ~ 318Hz)
    let m = 0.0005;
    let r = 0.0001; // Extremely low damping
    let k = 2000.0;

    plugin.sample_rate = sample_rate;

    println!(
        "Starting simulation for Large Amplitude (m={}, k={}, r={})...",
        m, k, r
    );

    for i in 0..num_samples {
        // Very strong breath pressure
        plugin.v_breath = if i < 100 {
            (i as f32 / 100.0) * 100.0
        } else {
            100.0
        };
        data.push((i as f32, custom_step(&mut plugin, m, r, k)));
    }

    Chart::new(180, 60, 0.0, num_samples as f32)
        .lineplot(&Shape::Lines(&data))
        .display();

    Ok(())
}
