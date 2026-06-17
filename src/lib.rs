use nih_plug::prelude::*;
use std::sync::Arc;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

const RHO: f32 = 1.2;
const C_SPEED: f32 = 340.0;
const RADIUS: f32 = 0.015;

pub struct Bore {
    pub delay_buffer: Vec<f32>,
    pub pointer: usize,
    pub z0: f32,
    pub m_e: f32,
    pub r_loss: f32,
    pub v_prev: f32,
}

impl Bore {
    pub fn new(target_freq: f32, sample_rate: f32, boundary_type: BoundaryType) -> Self {
        let wavelength_factor = match boundary_type {
            BoundaryType::NonlinearDissipative => 2.0,
            BoundaryType::Fixed => 1.0,
        };
        let delay_samples = (sample_rate / (target_freq * wavelength_factor)).round() as usize;
        let s = std::f32::consts::PI * RADIUS * RADIUS;
        let z0 = (RHO * C_SPEED) / s;
        let delta_l = 0.613 * RADIUS;
        let m_e = RHO * s * delta_l;
        let r_loss = 0.5 * RHO * s;

        Self {
            delay_buffer: vec![0.0; delay_samples],
            pointer: 0,
            z0,
            m_e,
            r_loss,
            v_prev: 0.0,
        }
    }

    pub fn set_frequency(
        &mut self,
        target_freq: f32,
        sample_rate: f32,
        boundary_type: BoundaryType,
    ) {
        let wavelength_factor = match boundary_type {
            BoundaryType::NonlinearDissipative => 2.0,
            BoundaryType::Fixed => 1.0,
        };
        let delay_samples = (sample_rate / (target_freq * wavelength_factor)).round() as usize;
        if delay_samples != self.delay_buffer.len() && delay_samples > 0 {
            self.delay_buffer.resize(delay_samples, 0.0);
            if self.pointer >= delay_samples {
                self.pointer = 0;
            }
        }
    }

    pub fn step(&mut self, p_plus_in: f32, t: f32, boundary_type: BoundaryType) -> f32 {
        if self.delay_buffer.is_empty() {
            return 0.0;
        }

        let p_minus_out = self.delay_buffer[self.pointer];

        let p_minus_reflected = match boundary_type {
            BoundaryType::NonlinearDissipative => {
                let s = std::f32::consts::PI * RADIUS * RADIUS;
                let b_bc = (self.m_e / t) + (s * self.z0);
                let c_bc = 2.0 * s * p_plus_in + (self.m_e / t) * self.v_prev;

                let discriminant = (b_bc * b_bc + 4.0 * self.r_loss * c_bc).max(0.0);
                let v_current = (-b_bc + discriminant.sqrt()) / (2.0 * self.r_loss);

                let res = p_plus_in - self.z0 * v_current;
                self.v_prev = v_current;
                res
            }
            BoundaryType::Fixed => p_plus_in,
        };

        self.delay_buffer[self.pointer] = p_minus_reflected;
        self.pointer = (self.pointer + 1) % self.delay_buffer.len();

        p_minus_out
    }

    pub fn reset(&mut self) {
        for val in self.delay_buffer.iter_mut() {
            *val = 0.0;
        }
        self.pointer = 0;
        self.v_prev = 0.0;
    }
}

pub struct Reed {
    pub x_prev1: f32,
    pub x_prev2: f32,
    pub f_prev1: f32,
    pub f_prev2: f32,
    pub vf_prev1: f32,
}

impl Reed {
    pub fn new() -> Self {
        Self {
            x_prev1: 0.0,
            x_prev2: 0.0,
            f_prev1: 0.0,
            f_prev2: 0.0,
            vf_prev1: 0.0,
        }
    }

    pub fn step(
        &mut self,
        p_total: f32,
        oscillation_type: OscillationType,
        sample_rate: f32,
        params: &AerothesisParams,
        v_bite: f32,
    ) -> (f32, f32) {
        let t = 1.0 / sample_rate;
        const EPS: f32 = 1e-5;

        let gap_curr = (2.0 - self.x_prev1).max(EPS);
        let b_curr = RHO / (4.0 * gap_curr * gap_curr);
        let a_fluid = (RHO * params.reed_length.value()) / t;

        let c_prev = p_total - b_curr * (self.vf_prev1 * self.vf_prev1);
        let discriminant =
            (a_fluid * a_fluid + 4.0 * b_curr * (a_fluid * self.vf_prev1 + c_prev)).max(0.0);
        let vf_current = (-a_fluid + discriminant.sqrt()) / (2.0 * b_curr);

        let mut f_current = if self.x_prev1 >= 2.0 {
            0.0
        } else {
            0.5 * RHO * (vf_current * vf_current) * gap_curr
        };

        if oscillation_type == OscillationType::LipReed {
            f_current = -f_current;
        }

        let m = params.base_mass.value() * (1.0 - params.bite_mass_scale.value() * v_bite);
        let r = params.base_damping.value() * (1.0 + params.bite_damping_scale.value() * v_bite);
        let k =
            params.base_stiffness.value() * (1.0 + params.bite_stiffness_scale.value() * v_bite);

        let b0 = t * t;
        let b1 = 2.0 * t * t;
        let b2 = t * t;

        let a0 = 4.0 * m + 2.0 * r * t + k * t * t;
        let a1 = -8.0 * m + 2.0 * k * t * t;
        let a2 = 4.0 * m - 2.0 * r * t + k * t * t;

        let mut x_n = (b0 * f_current + b1 * self.f_prev1 + b2 * self.f_prev2
            - a1 * self.x_prev1
            - a2 * self.x_prev2)
            / a0;

        if x_n >= 2.0 {
            x_n = 2.0;
        } else if x_n < 0.0 {
            x_n = 0.0;
        }

        self.x_prev2 = self.x_prev1;
        self.x_prev1 = x_n;
        self.f_prev2 = self.f_prev1;
        self.f_prev1 = f_current;
        self.vf_prev1 = vf_current;

        (x_n, vf_current)
    }

    pub fn reset(&mut self) {
        self.x_prev1 = 0.0;
        self.x_prev2 = 0.0;
        self.f_prev1 = 0.0;
        self.f_prev2 = 0.0;
        self.vf_prev1 = 0.0;
    }
}

pub struct Aerothesis {
    pub params: Arc<AerothesisParams>,

    pub x_prev: f32,
    pub x_prev2: f32,

    pub v_prev: f32,

    pub f_prev: f32,
    pub f_prev2: f32,
    pub sample_rate: f32,

    pub v_breath: f32,
    pub v_bite: f32,

    pub v_fluid_prev: f32,

    pub bore: Bore,
    pub reed: Reed,
    pub p_minus: f32,
    pub current_frequency: f32,
}

#[derive(Enum, PartialEq, Clone, Copy)]
pub enum OscillationType {
    #[name = "Single Reed"]
    SingleReed,
    #[name = "Lip Reed"]
    LipReed,
}

#[derive(Enum, PartialEq, Clone, Copy)]
pub enum BoundaryType {
    #[name = "Nonlinear Dissipative"]
    NonlinearDissipative,
    #[name = "Fixed End"]
    Fixed,
}

#[derive(Params)]
pub struct AerothesisParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "output_gain"]
    pub output_gain: FloatParam,

    #[id = "oscillation_type"]
    pub oscillation_type: EnumParam<OscillationType>,

    #[id = "boundary_type"]
    pub boundary_type: EnumParam<BoundaryType>,

    #[id = "ReedLength"]
    pub reed_length: FloatParam,

    #[id = "base_mass"]
    pub base_mass: FloatParam,
    #[id = "bite_mass_scale"]
    pub bite_mass_scale: FloatParam,
    #[id = "base_stiffness"]
    pub base_stiffness: FloatParam,
    #[id = "bite_stiffness_scale"]
    pub bite_stiffness_scale: FloatParam,
    #[id = "base_damping"]
    pub base_damping: FloatParam,
    #[id = "bite_damping_scale"]
    pub bite_damping_scale: FloatParam,
    #[id = "breath_damping"]
    pub breath_damping: FloatParam,
    #[id = "pressure_scale"]
    pub pressure_scale: FloatParam,
    #[id = "feedback_gain"]
    pub feedback_gain: FloatParam,

    #[id = "breath_cc"]
    pub breath_cc: IntParam,
    #[id = "bite_cc"]
    pub bite_cc: IntParam,
}

impl Default for Aerothesis {
    fn default() -> Self {
        let sample_rate = 44100.0;
        let target_freq = 220.0;
        let params = Arc::new(AerothesisParams::default());
        Self {
            bore: Bore::new(target_freq, sample_rate, params.boundary_type.value()),
            params,
            x_prev: 0.0,
            x_prev2: 0.0,
            v_prev: 0.0,
            f_prev: 0.0,
            f_prev2: 0.0,
            sample_rate,

            v_breath: 0.0,
            v_bite: 0.0,
            v_fluid_prev: 0.0,

            reed: Reed::new(),
            p_minus: 0.0,
            current_frequency: target_freq,
        }
    }
}

impl Default for AerothesisParams {
    fn default() -> Self {
        Self {
            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    // This makes the range appear as if it was linear when displaying the values as
                    // decibels
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            // Because the gain parameter is stored as linear gain instead of storing the value as
            // decibels, we need logarithmic smoothing
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            // There are many predefined formatters we can use here. If the gain was stored as
            // decibels instead of as a linear gain value, we could have also used the
            // `.with_step_size(0.1)` function to get internal rounding.
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            output_gain: FloatParam::new(
                "Output Gain",
                0.01,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: 0.2,
                },
            ),

            oscillation_type: EnumParam::new("Oscillation Type", OscillationType::SingleReed),

            boundary_type: EnumParam::new("Boundary Type", BoundaryType::NonlinearDissipative),

            reed_length: FloatParam::new(
                "Reed Length",
                0.01,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 0.1,
                    factor: 0.2,
                },
            ),

            base_mass: FloatParam::new(
                "Base Mass",
                0.0005,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 0.01,
                    factor: 0.2,
                },
            ),
            bite_mass_scale: FloatParam::new(
                "Bite Mass Reduction",
                0.5,
                FloatRange::Linear { min: 0.0, max: 0.9 },
            ),
            base_stiffness: FloatParam::new(
                "Base Stiffness",
                2000.0,
                FloatRange::Skewed {
                    min: 100.0,
                    max: 20000.0,
                    factor: 0.3,
                },
            ),
            bite_stiffness_scale: FloatParam::new(
                "Bite Stiffness Incr.",
                2.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            ),
            base_damping: FloatParam::new(
                "Base Damping",
                0.001,
                FloatRange::Skewed {
                    min: 0.0001,
                    max: 0.1,
                    factor: 0.2,
                },
            ),
            bite_damping_scale: FloatParam::new(
                "Bite Damping Incr.",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            ),
            breath_damping: FloatParam::new(
                "Breath Damping",
                0.1,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            pressure_scale: FloatParam::new(
                "Pressure Gain",
                50.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 200.0,
                },
            ),
            feedback_gain: FloatParam::new(
                "Feedback Gain",
                0.1,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            breath_cc: IntParam::new("Breath CC", 2, IntRange::Linear { min: 0, max: 127 }),
            bite_cc: IntParam::new("Bite CC", 11, IntRange::Linear { min: 0, max: 127 }),
        }
    }
}

impl Aerothesis {
    pub fn step(&mut self) -> f32 {
        let x_n = self.x();
        let v_n = self.v(x_n);

        self.x_prev2 = self.x_prev;
        self.x_prev = x_n;

        self.f_prev2 = self.f_prev;
        self.f_prev = self.f();

        self.v_prev = v_n;
        self.v_fluid_prev = self.vf();

        x_n
    }

    pub fn m(&self) -> f32 {
        self.params.base_mass.value() * (1.0 - self.params.bite_mass_scale.value() * self.v_bite)
    }

    pub fn r(&self) -> f32 {
        self.params.base_damping.value()
            * (1.0 + self.params.bite_damping_scale.value() * self.v_bite)
    }

    pub fn k(&self) -> f32 {
        self.params.base_stiffness.value()
            * (1.0 + self.params.bite_stiffness_scale.value() * self.v_bite)
    }
    pub fn vf(&self) -> f32 {
        const EPS: f32 = 1e-5;
        const RHO: f32 = 1.2;
        let t = 1.0 / self.sample_rate;

        let a_fluid = (RHO * self.params.reed_length.value()) / t;

        let gap_prev = (2.0 - self.x_prev).clamp(EPS, 2.0);
        let b_prev = RHO / (4.0 * (gap_prev * gap_prev));
        let c_prev = self.v_breath - b_prev * (self.v_fluid_prev * self.v_fluid_prev);

        // Current gap is also based on x_prev in this discrete model for stability
        let gap_curr = (2.0 - self.x_prev).clamp(EPS, 2.0);
        let b_curr = RHO / (4.0 * (gap_curr * gap_curr));

        if gap_curr <= EPS {
            0.0
        } else {
            let discriminant = (a_fluid * a_fluid
                + 4.0 * b_curr * (a_fluid * self.v_fluid_prev + c_prev))
                .max(0.0);
            let numerator = -a_fluid + discriminant.sqrt();
            numerator / (2.0 * b_curr)
        }
    }
    pub fn f(&self) -> f32 {
        const EPS: f32 = 1e-5;
        const RHO: f32 = 1.2;

        let gap_curr = (2.0 - self.x_prev).clamp(EPS, 2.0);

        let v_fluid_current = self.vf();

        let f_current = if self.x_prev >= 2.0 {
            0.0
        } else {
            0.5 * RHO * (v_fluid_current * v_fluid_current) * gap_curr
        };
        f_current
    }
    pub fn x(&self) -> f32 {
        let m = self.m();
        let r = self.r();
        let k = self.k();
        let t = 1.0 / self.sample_rate;
        let f_current = self.f();

        let b0 = t * t;
        let b1 = 2.0 * t * t;
        let b2 = t * t;

        let a0 = 4.0 * m + 2.0 * r * t + k * t * t;
        let a1 = -8.0 * m + 2.0 * k * t * t;
        let a2 = 4.0 * m - 2.0 * r * t + k * t * t;

        ((b0 * f_current + b1 * self.f_prev + b2 * self.f_prev2
            - a1 * self.x_prev
            - a2 * self.x_prev2)
            / a0)
            .clamp(0.0, 2.0)
    }

    pub fn v(&self, x: f32) -> f32 {
        let t = 1.0 / self.sample_rate;
        (2.0 / t) * (x - self.x_prev) - self.v_prev
    }
    fn equilibrium_offset(&self) -> f32 {
        let f = self.f();
        let k = self.k();
        if k > 0.0 {
            (f / k).clamp(0.0, 1.8)
        } else {
            0.0
        }
    }
}

impl Plugin for Aerothesis {
    const NAME: &'static str = "Aerothesis";
    const VENDOR: &'static str = "Minerva_Juppiter";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "aerothesis@minervajuppiter.net";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.bore = Bore::new(
            self.current_frequency,
            self.sample_rate,
            self.params.boundary_type.value(),
        );
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
        self.bore.reset();
        self.reed.reset();
        self.p_minus = 0.0;
        self.v_breath = 0.0;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::MidiCC {
                    timing: _,
                    channel: _,
                    cc,
                    value,
                } => {
                    if cc as i32 == self.params.breath_cc.value() {
                        self.v_breath = value * self.params.pressure_scale.value();
                    } else if cc as i32 == self.params.bite_cc.value() {
                        self.v_bite = value;
                    }
                }
                NoteEvent::NoteOn { note, .. } => {
                    self.current_frequency = util::midi_note_to_freq(note);
                    self.bore.set_frequency(
                        self.current_frequency,
                        self.sample_rate,
                        self.params.boundary_type.value(),
                    );
                }
                _ => (),
            }
        }

        let t = 1.0 / self.sample_rate;

        for channel_samples in buffer.iter_samples() {
            let gain = self.params.gain.smoothed.next();

            let p_total = self.v_breath + self.p_minus;
            let (x_n, vf_n) = self.reed.step(
                p_total,
                self.params.oscillation_type.value(),
                self.sample_rate,
                &self.params,
                self.v_bite,
            );

            let gap = (2.0 - x_n).max(1e-5);
            let u = vf_n * gap;
            let p_mouth = self.p_minus + self.bore.z0 * u;
            let p_plus = p_mouth - self.p_minus;

            self.p_minus = self.bore.step(p_plus, t, self.params.boundary_type.value());

            let mut output = (p_mouth * self.params.output_gain.value()).clamp(-1.0, 1.0);

            if self.v_breath < 1e-4 {
                output = 0.0;
                self.bore.reset();
                self.reed.reset();
                self.p_minus = 0.0;
            }

            let final_output = output * gain;

            for sample in channel_samples {
                *sample = final_output;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Aerothesis {
    const CLAP_ID: &'static str = "com.your-domain.aerothesis";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A short description of your plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::NoteDetector,
        ClapFeature::Instrument,
    ];
}

// impl Vst3Plugin for Aerothesis {
//     const VST3_CLASS_ID: [u8; 16] = *b"Exactly16Chars!!";

//     // And also don't forget to change these categories
//     const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
//         &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
// }

nih_export_clap!(Aerothesis);
// nih_export_vst3!(Aerothesis);
