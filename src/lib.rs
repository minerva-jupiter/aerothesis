use nih_plug::prelude::*;
use std::{collections::VecDeque, sync::Arc};

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

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

    pub x_history: VecDeque<f32>,

    pub note_frequency: f32,
}

#[derive(Enum, PartialEq, Clone, Copy)]
pub enum InstrumentType {
    #[name = "Single Reed"]
    SingleReed,
    #[name = "Rip Reed"]
    LipReed,
}

#[derive(Enum, PartialEq, Clone, Copy)]
pub enum ResonanceType {
    #[name = "Open Pipe"]
    OpenPipe,
    #[name = "Closed Pipe"]
    ClosedPipe,
}

#[derive(Params)]
pub struct AerothesisParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "instrument_type"]
    pub instrument_type: EnumParam<InstrumentType>,

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

    #[id = "resonance_type"]
    pub resonance_type: EnumParam<ResonanceType>,

    #[id = "resonance_decay"]
    pub resonance_decay: FloatParam,
}

impl Default for Aerothesis {
    fn default() -> Self {
        Self {
            params: Arc::new(AerothesisParams::default()),
            x_prev: 0.0,
            x_prev2: 0.0,
            v_prev: 0.0,
            f_prev: 0.0,
            f_prev2: 0.0,
            sample_rate: 44100.0,

            v_breath: 0.1,
            v_bite: 0.0,
            v_fluid_prev: 0.0,

            x_history: VecDeque::new(),

            note_frequency: 0.0,
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

            instrument_type: EnumParam::new("Instrument Type", InstrumentType::SingleReed),

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

            resonance_type: EnumParam::new("Resonance Type", ResonanceType::OpenPipe),
            resonance_decay: FloatParam::new(
                "Resonance Decay",
                0.9,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: 0.8,
                },
            ),
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

    pub fn resonance(&mut self) -> f32 {
        let x_n = self.step();
        let x_oscillator = x_n - self.equilibrium_offset();

        let resonance = if self.resonance_delay_samples() > self.x_history.len() as f32 {
            0.0
        } else {
            let decay: f32 = if self.params.resonance_type.value() == ResonanceType::OpenPipe {
                1.0
            } else {
                -1.0
            } * self.params.resonance_decay.value();
            let x_delay = self.x_history.pop_front().unwrap_or(0.0);
            decay * x_delay
        };

        let x_current = x_oscillator + resonance;
        self.x_history.push_back(x_current);

        x_current
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
    fn resonance_delay_samples(&self) -> f32 {
        if self.params.resonance_type.value() == ResonanceType::OpenPipe {
            self.sample_rate / self.note_frequency
        } else {
            self.sample_rate / 2.0 / self.note_frequency
        }
    }
    fn avg_x_history(&self) -> f32 {
        self.x_history.iter().sum::<f32>() / self.x_history.len() as f32
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
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
        self.x_history.clear();
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
                NoteEvent::NoteOn {
                    timing: _,
                    voice_id: _,
                    channel: _,
                    note,
                    velocity: _,
                } => {
                    self.reset();
                    self.note_frequency = util::midi_note_to_freq(note);
                }
                _ => (),
            }
        }

        for channel_samples in buffer.iter_samples() {
            let gain = self.params.gain.smoothed.next();
            let x_current = self.resonance() - self.avg_x_history();

            for sample in channel_samples {
                *sample = (x_current * gain).clamp(-1.0, 1.0);
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
