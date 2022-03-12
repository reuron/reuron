use crate::constants::{GAS_CONSTANT, INVERSE_FARADAY};
use crate::dimension::{Interval, Kelvin, MilliVolts, Molar, Siemens, Volts};
use crate::neuron::solution::Solution;

/// The relative permeability of a channel to various ions.
/// These should add to 1.0.
#[derive(Clone, Debug)]
pub struct IonSelectivity {
    /// Sodium+.
    pub na: f32,
    /// Potasium+.
    pub k: f32,
    /// Calcium2+.
    pub ca: f32,
    /// Chloride-.
    pub cl: f32,
}

pub const K: IonSelectivity = IonSelectivity {
    na: 0.0,
    k: 1.0,
    ca: 0.0,
    cl: 0.0,
};

pub const NA: IonSelectivity = IonSelectivity {
    na: 1.0,
    k: 0.0,
    ca: 0.0,
    cl: 0.0,
};

pub const CA: IonSelectivity = IonSelectivity {
    na: 0.0,
    k: 0.0,
    ca: 1.0,
    cl: 0.0,
};

pub const CL: IonSelectivity = IonSelectivity {
    na: 0.0,
    k: 0.0,
    ca: 0.0,
    cl: 1.0,
};

pub fn reversal_potential(
    internal_concentration: &Molar,
    external_concentration: &Molar,
    temperature: &Kelvin,
    valence: i8,
) -> MilliVolts {
    let v = GAS_CONSTANT * INVERSE_FARADAY * temperature.0 / valence as f32
        * (external_concentration.0 / internal_concentration.0).ln();
    MilliVolts(v * 1000.0)
}

pub fn k_reversal(
    internal_solution: &Solution,
    external_solution: &Solution,
    temperature: &Kelvin,
) -> MilliVolts {
    reversal_potential(
        &internal_solution.k_concentration,
        &external_solution.k_concentration,
        temperature,
        1,
    )
}

pub fn na_reversal(
    internal_solution: &Solution,
    external_solution: &Solution,
    temperature: &Kelvin,
) -> MilliVolts {
    reversal_potential(
        &internal_solution.na_concentration,
        &external_solution.na_concentration,
        temperature,
        1,
    )
}

pub fn ca_reversal(
    internal_solution: &Solution,
    external_solution: &Solution,
    temperature: &Kelvin,
) -> MilliVolts {
    reversal_potential(
        &internal_solution.ca_concentration,
        &external_solution.ca_concentration,
        temperature,
        2,
    )
}

pub fn cl_reversal(
    internal_solution: &Solution,
    external_solution: &Solution,
    temperature: &Kelvin,
) -> MilliVolts {
    reversal_potential(
        &internal_solution.cl_concentration,
        &external_solution.cl_concentration,
        temperature,
        -1,
    )
}

impl IonSelectivity {
    pub fn normalize(&self) -> IonSelectivity {
        let sum = self.k + self.na + self.ca + self.cl;
        IonSelectivity {
            k: self.k / sum,
            na: self.na / sum,
            ca: self.ca / sum,
            cl: self.cl / sum,
        }
    }
}

/// State of the voltage-gated conductance, such as the conductance of
/// a voltage-gated sodium channel or a voltage-gated potassium channel.
#[derive(Clone, Debug)]
pub struct Channel {
    /// State of the activation gates.
    pub activation: Option<GateState>,
    /// State of the inactivation gates.
    pub inactivation: Option<GateState>,
    /// The ion this channel is permeable to.
    pub ion_selectivity: IonSelectivity,
}

impl Channel {
    /// Advance the channel conduction state for the activation and inactivation
    /// magnitudes.
    pub fn step(&mut self, membrane_potential: &MilliVolts, interval: &Interval) {
        self.activation
            .iter_mut()
            .for_each(|activation| activation.step(membrane_potential, interval));
        self.inactivation
            .iter_mut()
            .for_each(|inactivation| inactivation.step(membrane_potential, interval));
    }

    /// The
    pub fn conductance_coefficient(&self) -> f32 {
        let activation_coefficient = self.activation.as_ref().map_or(1.0, |gate_state| {
            gate_state
                .magnitude
                .powi(gate_state.parameters.gates as i32)
        });
        let inactivation_coefficient = self.inactivation.as_ref().map_or(1.0, |gate_state| {
            gate_state
                .magnitude
                .powi(gate_state.parameters.gates as i32)
        });
        activation_coefficient * inactivation_coefficient
    }
}

#[derive(Clone, Debug)]
pub struct ChannelBuilder {
    pub activation_parameters: Option<Gating>,
    pub inactivation_parameters: Option<Gating>,
    pub ion_selectivity: IonSelectivity,
}

impl ChannelBuilder {
    /// Construct a new conductance state from a set of activation and
    /// inactivation parameters. Choose an initial state for the activation and
    /// inactivation gates by setting them to their steady-state levels.
    pub fn build(self, initial_membrane_potential: &MilliVolts) -> Channel {
        let activation = self.activation_parameters.map(|parameters| {
            let magnitude = parameters
                .steady_state_magnitude
                .steady_state(initial_membrane_potential);
            GateState {
                magnitude,
                parameters: parameters,
            }
        });
        let inactivation = self.inactivation_parameters.map(|parameters| {
            let magnitude = parameters
                .steady_state_magnitude
                .steady_state(initial_membrane_potential);
            GateState {
                magnitude,
                parameters: parameters,
            }
        });
        Channel {
            activation,
            inactivation,
            ion_selectivity: self.ion_selectivity.normalize(),
        }
    }
}

/// The state for a particular type of game (either the activation or
/// inactivation gate).
#[derive(Clone, Debug)]
pub struct GateState {
    /// The current magnitude of tha conductance component. 'm', 'n' or 'h' in
    /// the Hodgkin-Huxley model.
    pub magnitude: f32,
    /// The parameters determining how the magnitutde evolves with time and
    /// membrane voltage.
    pub parameters: Gating,
}

impl GateState {
    /// Update the activation/inactivation state by computing (a) the
    /// steady-state value at the current membrane voltage, and (b) the time
    /// constant, tau, at the current membrane voltage.
    pub fn step(&mut self, membrane_potential: &MilliVolts, interval: &Interval) {
        let v_inf = self
            .parameters
            .steady_state_magnitude
            .steady_state(membrane_potential);
        let tau = self.parameters.time_constant.tau(membrane_potential);
        let df_dt = (v_inf - self.magnitude) / tau;
        self.magnitude = self.magnitude + df_dt * interval.0;
    }
}

/// The confuration for a single type of gate in a single channel.
#[derive(Clone, Debug)]
pub struct Gating {
    /// The number of such gates in each channel. For instance, the 3
    /// activation gates of a potassium channel, or the 1 inactivation
    /// gate of a sodium channel.
    pub gates: u8,
    pub steady_state_magnitude: Magnitude,
    pub time_constant: TimeConstant,
}

#[derive(Clone, Debug)]
pub struct Magnitude {
    pub v_at_half_max: MilliVolts,
    pub slope: f32,
}

impl Magnitude {
    pub fn steady_state(&self, v: &MilliVolts) -> f32 {
        1.0 / (1.0 + ((self.v_at_half_max.0 - v.0) / self.slope).exp())
    }
}

#[derive(Clone, Debug)]
pub struct TimeConstant {
    pub v_at_max_tau: MilliVolts,
    pub c_base: f32,
    pub c_amp: f32,
    pub sigma: f32,
}

impl TimeConstant {
    pub fn tau(&self, v: &MilliVolts) -> f32 {
        let numerator = -1.0 * (self.v_at_max_tau.0 - v.0).powi(2);
        let denominator = self.sigma.powi(2);
        self.c_base + self.c_amp * (numerator / denominator).exp()
    }
}

pub mod common_channels {

    pub mod giant_squid {
        use crate::dimension::MilliVolts;
        use crate::neuron::channel::*;

        /// The Giant Squid axon's Na+ channel.
        pub const NA_CHANNEL: ChannelBuilder = ChannelBuilder {
            ion_selectivity: NA,
            activation_parameters: Some(Gating {
                gates: 3,
                steady_state_magnitude: Magnitude {
                    v_at_half_max: MilliVolts(-40.0),
                    slope: 15.0,
                },
                time_constant: TimeConstant {
                    v_at_max_tau: MilliVolts(-38.0),
                    c_base: 0.04e-3,
                    c_amp: 0.46e-3,
                    sigma: 30.0,
                },
            }),
            inactivation_parameters: Some(Gating {
                gates: 1,
                steady_state_magnitude: Magnitude {
                    v_at_half_max: MilliVolts(-62.0),
                    slope: -7.0,
                },
                time_constant: TimeConstant {
                    v_at_max_tau: MilliVolts(-67.0),
                    c_base: 0.0012, // TODO are these right?
                    c_amp: 0.0074,
                    sigma: 20.0,
                },
            }),
        };

        /// The Giant Squid axon's K+ rectifying channel.
        pub const K_CHANNEL: ChannelBuilder = ChannelBuilder {
            ion_selectivity: K,
            activation_parameters: Some(Gating {
                gates: 4,
                steady_state_magnitude: Magnitude {
                    v_at_half_max: MilliVolts(-53.0),
                    slope: 15.0,
                },
                time_constant: TimeConstant {
                    v_at_max_tau: MilliVolts(-79.0),
                    c_base: 1.1e-3,
                    c_amp: 4.7e-3,
                    sigma: 50.0,
                },
            }),
            inactivation_parameters: None,
        };

        /// The Gaint Squid axon's leak current.
        pub const LEAK_CHANNEL: ChannelBuilder = ChannelBuilder {
            ion_selectivity: CL,
            activation_parameters: None,
            inactivation_parameters: None,
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::constants::*;
    use crate::dimension::*;
    use crate::neuron::channel::*;
    use crate::neuron::solution::*;

    #[test]
    fn activations_tend_toward_v_inf() {
        let builder_voltage = MilliVolts(0.0);
        let membrane_potential = MilliVolts(-60.0);
        let mut na_channel = crate::neuron::channel::common_channels::giant_squid::NA_CHANNEL
            .build(&builder_voltage);
        let interval = Interval(0.01);
        for i in 0..1000 {
            na_channel.step(&membrane_potential, &interval);
        }
        let expected_magnitude = Magnitude {
            v_at_half_max: MilliVolts(-40.0),
            slope: 15.0,
        }
        .steady_state(&membrane_potential);
        assert!((na_channel.activation.unwrap().magnitude - expected_magnitude).abs() < EPSILON);
    }

    #[test]
    fn na_channel_inactivates() {
        let builder_voltage = MilliVolts(-60.0);
        let membrane_potential = MilliVolts(80.0);
        let mut na_channel = crate::neuron::channel::common_channels::giant_squid::NA_CHANNEL
            .build(&builder_voltage);
        let interval = Interval(0.001);
        for n in 0..1000 {
            na_channel.step(&membrane_potential, &interval);
        }
        assert!(na_channel.inactivation.unwrap().magnitude < 0.001);
    }

    #[test]
    pub fn reversal_potentials() {
        let actual = k_reversal(&EXAMPLE_CYTOPLASM, &INTERSTICIAL_FLUID, &BODY_TEMPERATURE);
        let expected = MilliVolts(-89.01071);
        assert!((actual.0 - expected.0).abs() < EPSILON);

        let actual = na_reversal(&EXAMPLE_CYTOPLASM, &INTERSTICIAL_FLUID, &BODY_TEMPERATURE);
        let expected = MilliVolts(89.948074);
        assert!((actual.0 - expected.0).abs() < EPSILON);

        let actual = cl_reversal(&EXAMPLE_CYTOPLASM, &INTERSTICIAL_FLUID, &BODY_TEMPERATURE);
        let expected = MilliVolts(-88.52939);
        assert!((actual.0 - expected.0).abs() < EPSILON);

        let actual = ca_reversal(&EXAMPLE_CYTOPLASM, &INTERSTICIAL_FLUID, &BODY_TEMPERATURE);
        let expected = MilliVolts(135.25258);
        assert!((actual.0 - expected.0).abs() < EPSILON);
    }
}
