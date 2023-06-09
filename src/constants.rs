use crate::dimension::Kelvin;

pub const GAS_CONSTANT: f32 = 8.314;
pub const BODY_TEMPERATURE: Kelvin = Kelvin(310.0);
pub const INVERSE_FARADAY: f32 = 1.0 / 96485.3;

pub const CONDUCTANCE_PER_SQUARE_CM: f32 = 10.000; // TODO: Figure this out.

pub const EPSILON: f32 = 1e-3;

pub const SIMULATION_STEPS_PER_FRAME: usize = 100;
