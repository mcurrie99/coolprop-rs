use std::cell::Cell;
use std::marker::PhantomData;
use std::os::raw::{c_char, c_long};

use crate::{
    buffer_to_string, c_string, input_pair_index_locked, param_index_locked, validate_scalar,
    with_coolprop, with_error_buffer, Error, InputPair, Parameter, Phase, PhaseSpecifier, Result,
    SaturatedState,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommonOutputs {
    pub temperature: f64,
    pub pressure: f64,
    pub molar_density: f64,
    pub molar_enthalpy: f64,
    pub molar_entropy: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateUpdate {
    pub input_pair: InputPair,
    pub value1: f64,
    pub value2: f64,
}

impl StateUpdate {
    pub const fn new(input_pair: InputPair, value1: f64, value2: f64) -> Self {
        Self {
            input_pair,
            value1,
            value2,
        }
    }

    pub const fn pressure_temperature(pressure: f64, temperature: f64) -> Self {
        Self::new(InputPair::PressureTemperature, pressure, temperature)
    }

    pub const fn pressure_quality(pressure: f64, quality: f64) -> Self {
        Self::new(InputPair::PressureQuality, pressure, quality)
    }

    pub const fn quality_temperature(quality: f64, temperature: f64) -> Self {
        Self::new(InputPair::QualityTemperature, quality, temperature)
    }

    pub const fn mass_density_temperature(density: f64, temperature: f64) -> Self {
        Self::new(InputPair::MassDensityTemperature, density, temperature)
    }

    pub const fn mass_enthalpy_pressure(enthalpy: f64, pressure: f64) -> Self {
        Self::new(InputPair::MassEnthalpyPressure, enthalpy, pressure)
    }

    pub const fn pressure_mass_entropy(pressure: f64, entropy: f64) -> Self {
        Self::new(InputPair::PressureMassEntropy, pressure, entropy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BatchFiveOutputs {
    pub output1: Vec<f64>,
    pub output2: Vec<f64>,
    pub output3: Vec<f64>,
    pub output4: Vec<f64>,
    pub output5: Vec<f64>,
}

impl BatchFiveOutputs {
    pub fn row(&self, index: usize) -> Option<[f64; 5]> {
        Some([
            *self.output1.get(index)?,
            *self.output2.get(index)?,
            *self.output3.get(index)?,
            *self.output4.get(index)?,
            *self.output5.get(index)?,
        ])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhaseEnvelopeData {
    pub points: Vec<PhaseEnvelopePoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhaseEnvelopePoint {
    pub temperature: f64,
    pub pressure: f64,
    pub vapor_molar_density: f64,
    pub liquid_molar_density: f64,
    pub liquid_mole_fractions: Vec<f64>,
    pub vapor_mole_fractions: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpinodalPoint {
    pub tau: f64,
    pub delta: f64,
    pub m1: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CriticalPoint {
    pub temperature: f64,
    pub pressure: f64,
    pub molar_density: f64,
    pub stable: bool,
}

#[derive(Debug)]
pub struct AbstractState {
    handle: c_long,
    backend: String,
    fluids: String,
    _not_sync: PhantomData<Cell<()>>,
}

impl AbstractState {
    pub fn new(backend: impl AsRef<str>, fluids: impl AsRef<str>) -> Result<Self> {
        let backend_string = backend.as_ref().to_owned();
        let fluids_string = fluids.as_ref().to_owned();
        let backend = c_string(&backend_string, "backend")?;
        let fluids = c_string(&fluids_string, "fluids")?;

        let handle = with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_factory",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_factory(
                        backend.as_ptr(),
                        fluids.as_ptr(),
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })?;

        if handle < 0 {
            return Err(Error::coolprop_message(
                "AbstractState_factory returned an invalid handle",
            ));
        }

        Ok(Self {
            handle,
            backend: backend_string,
            fluids: fluids_string,
            _not_sync: PhantomData,
        })
    }

    pub fn backend(&self) -> &str {
        &self.backend
    }

    pub fn fluids(&self) -> &str {
        &self.fluids
    }

    pub fn raw_handle(&self) -> c_long {
        self.handle
    }

    pub fn update(&mut self, input_pair: InputPair, value1: f64, value2: f64) -> Result<()> {
        self.update_by_name(input_pair.as_str(), value1, value2)
    }

    pub fn update_state(&mut self, update: StateUpdate) -> Result<()> {
        self.update(update.input_pair, update.value1, update.value2)
    }

    pub fn update_by_name(
        &mut self,
        input_pair: impl AsRef<str>,
        value1: f64,
        value2: f64,
    ) -> Result<()> {
        let input_pair = input_pair.as_ref();

        with_coolprop(|coolprop| {
            let input_pair = input_pair_index_locked(coolprop, input_pair)?;
            with_error_buffer(
                "AbstractState_update",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_update(
                        self.handle,
                        input_pair,
                        value1,
                        value2,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })
    }

    pub fn keyed_output(&mut self, parameter: Parameter) -> Result<f64> {
        self.keyed_output_by_name(parameter.as_str())
    }

    pub fn keyed_output_by_name(&mut self, parameter: impl AsRef<str>) -> Result<f64> {
        let parameter = parameter.as_ref();

        with_coolprop(|coolprop| {
            let parameter = param_index_locked(coolprop, parameter)?;
            let value = with_error_buffer(
                "AbstractState_keyed_output",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_keyed_output(
                        self.handle,
                        parameter,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )?;
            validate_scalar(coolprop, "AbstractState_keyed_output", value)
        })
    }

    pub fn temperature(&mut self) -> Result<f64> {
        self.keyed_output(Parameter::Temperature)
    }

    pub fn pressure(&mut self) -> Result<f64> {
        self.keyed_output(Parameter::Pressure)
    }

    pub fn mass_density(&mut self) -> Result<f64> {
        self.keyed_output(Parameter::MassDensity)
    }

    pub fn molar_density(&mut self) -> Result<f64> {
        self.keyed_output(Parameter::MolarDensity)
    }

    pub fn mass_enthalpy(&mut self) -> Result<f64> {
        self.keyed_output(Parameter::MassEnthalpy)
    }

    pub fn molar_enthalpy(&mut self) -> Result<f64> {
        self.keyed_output(Parameter::MolarEnthalpy)
    }

    pub fn mass_entropy(&mut self) -> Result<f64> {
        self.keyed_output(Parameter::MassEntropy)
    }

    pub fn molar_entropy(&mut self) -> Result<f64> {
        self.keyed_output(Parameter::MolarEntropy)
    }

    pub fn quality(&mut self) -> Result<f64> {
        self.keyed_output(Parameter::Quality)
    }

    pub fn viscosity(&mut self) -> Result<f64> {
        self.keyed_output(Parameter::Viscosity)
    }

    pub fn conductivity(&mut self) -> Result<f64> {
        self.keyed_output(Parameter::Conductivity)
    }

    pub fn speed_of_sound(&mut self) -> Result<f64> {
        self.keyed_output(Parameter::SpeedOfSound)
    }

    pub fn surface_tension(&mut self) -> Result<f64> {
        self.keyed_output(Parameter::SurfaceTension)
    }

    pub fn common_outputs(
        &mut self,
        input_pair: InputPair,
        value1: &[f64],
        value2: &[f64],
    ) -> Result<Vec<CommonOutputs>> {
        self.common_outputs_by_name(input_pair.as_str(), value1, value2)
    }

    pub fn common_outputs_by_name(
        &mut self,
        input_pair: impl AsRef<str>,
        value1: &[f64],
        value2: &[f64],
    ) -> Result<Vec<CommonOutputs>> {
        if value1.len() != value2.len() {
            return Err(Error::coolprop_message(format!(
                "value1 length {} does not match value2 length {}",
                value1.len(),
                value2.len()
            )));
        }

        let input_pair = input_pair.as_ref();
        let len: c_long = value1.len().try_into().map_err(|_| Error::LengthOverflow {
            what: "AbstractState_update_and_common_out",
            len: value1.len(),
        })?;

        let mut temperature = vec![0.0; value1.len()];
        let mut pressure = vec![0.0; value1.len()];
        let mut molar_density = vec![0.0; value1.len()];
        let mut molar_enthalpy = vec![0.0; value1.len()];
        let mut molar_entropy = vec![0.0; value1.len()];

        with_coolprop(|coolprop| {
            let input_pair = input_pair_index_locked(coolprop, input_pair)?;
            with_error_buffer(
                "AbstractState_update_and_common_out",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_update_and_common_out(
                        self.handle,
                        input_pair,
                        value1.as_ptr(),
                        value2.as_ptr(),
                        len,
                        temperature.as_mut_ptr(),
                        pressure.as_mut_ptr(),
                        molar_density.as_mut_ptr(),
                        molar_enthalpy.as_mut_ptr(),
                        molar_entropy.as_mut_ptr(),
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })?;

        Ok((0..value1.len())
            .map(|index| CommonOutputs {
                temperature: temperature[index],
                pressure: pressure[index],
                molar_density: molar_density[index],
                molar_enthalpy: molar_enthalpy[index],
                molar_entropy: molar_entropy[index],
            })
            .collect())
    }

    pub fn update_and_output(
        &mut self,
        input_pair: InputPair,
        value1: &[f64],
        value2: &[f64],
        output: Parameter,
    ) -> Result<Vec<f64>> {
        self.update_and_output_by_name(input_pair.as_str(), value1, value2, output.as_str())
    }

    pub fn update_and_output_by_name(
        &mut self,
        input_pair: impl AsRef<str>,
        value1: &[f64],
        value2: &[f64],
        output: impl AsRef<str>,
    ) -> Result<Vec<f64>> {
        ensure_same_len("value1", value1.len(), "value2", value2.len())?;
        let input_pair = input_pair.as_ref();
        let output = output.as_ref();
        let len = usize_to_c_long("AbstractState_update_and_1_out", value1.len())?;
        let mut out = vec![0.0; value1.len()];

        with_coolprop(|coolprop| {
            let input_pair = input_pair_index_locked(coolprop, input_pair)?;
            let output = param_index_locked(coolprop, output)?;
            with_error_buffer(
                "AbstractState_update_and_1_out",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_update_and_1_out(
                        self.handle,
                        input_pair,
                        value1.as_ptr(),
                        value2.as_ptr(),
                        len,
                        output,
                        out.as_mut_ptr(),
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })?;

        Ok(out)
    }

    pub fn update_and_5_outputs(
        &mut self,
        input_pair: InputPair,
        value1: &[f64],
        value2: &[f64],
        outputs: [Parameter; 5],
    ) -> Result<BatchFiveOutputs> {
        self.update_and_5_outputs_by_name(
            input_pair.as_str(),
            value1,
            value2,
            [
                outputs[0].as_str(),
                outputs[1].as_str(),
                outputs[2].as_str(),
                outputs[3].as_str(),
                outputs[4].as_str(),
            ],
        )
    }

    pub fn update_and_5_outputs_by_name(
        &mut self,
        input_pair: impl AsRef<str>,
        value1: &[f64],
        value2: &[f64],
        outputs: [&str; 5],
    ) -> Result<BatchFiveOutputs> {
        ensure_same_len("value1", value1.len(), "value2", value2.len())?;
        let input_pair = input_pair.as_ref();
        let len = usize_to_c_long("AbstractState_update_and_5_out", value1.len())?;
        let mut out1 = vec![0.0; value1.len()];
        let mut out2 = vec![0.0; value1.len()];
        let mut out3 = vec![0.0; value1.len()];
        let mut out4 = vec![0.0; value1.len()];
        let mut out5 = vec![0.0; value1.len()];

        with_coolprop(|coolprop| {
            let input_pair = input_pair_index_locked(coolprop, input_pair)?;
            let mut output_indices = [
                param_index_locked(coolprop, outputs[0])?,
                param_index_locked(coolprop, outputs[1])?,
                param_index_locked(coolprop, outputs[2])?,
                param_index_locked(coolprop, outputs[3])?,
                param_index_locked(coolprop, outputs[4])?,
            ];
            with_error_buffer(
                "AbstractState_update_and_5_out",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_update_and_5_out(
                        self.handle,
                        input_pair,
                        value1.as_ptr(),
                        value2.as_ptr(),
                        len,
                        output_indices.as_mut_ptr(),
                        out1.as_mut_ptr(),
                        out2.as_mut_ptr(),
                        out3.as_mut_ptr(),
                        out4.as_mut_ptr(),
                        out5.as_mut_ptr(),
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })?;

        Ok(BatchFiveOutputs {
            output1: out1,
            output2: out2,
            output3: out3,
            output4: out4,
            output5: out5,
        })
    }

    pub fn set_fractions(&mut self, fractions: &[f64]) -> Result<()> {
        let len: c_long = fractions
            .len()
            .try_into()
            .map_err(|_| Error::LengthOverflow {
                what: "AbstractState_set_fractions",
                len: fractions.len(),
            })?;

        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_set_fractions",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_set_fractions(
                        self.handle,
                        fractions.as_ptr(),
                        len,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })
    }

    pub fn mole_fractions(&mut self) -> Result<Vec<f64>> {
        self.mole_fractions_with_capacity(64)
    }

    pub fn mole_fractions_with_capacity(&mut self, max_components: usize) -> Result<Vec<f64>> {
        let max_components = usize_to_c_long("AbstractState_get_mole_fractions", max_components)?;
        let mut fractions = vec![0.0; max_components as usize];
        let mut actual_len: c_long = 0;

        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_get_mole_fractions",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_get_mole_fractions(
                        self.handle,
                        fractions.as_mut_ptr(),
                        max_components,
                        &mut actual_len,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })?;

        fractions.truncate(c_long_to_usize("mole fractions", actual_len)?);
        Ok(fractions)
    }

    pub fn saturated_mole_fractions(&mut self, state: SaturatedState) -> Result<Vec<f64>> {
        self.saturated_mole_fractions_with_capacity(state, 64)
    }

    pub fn saturated_mole_fractions_with_capacity(
        &mut self,
        state: SaturatedState,
        max_components: usize,
    ) -> Result<Vec<f64>> {
        let state = c_string(state.as_str(), "saturated_state")?;
        let max_components =
            usize_to_c_long("AbstractState_get_mole_fractions_satState", max_components)?;
        let mut fractions = vec![0.0; max_components as usize];
        let mut actual_len: c_long = 0;

        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_get_mole_fractions_satState",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_get_mole_fractions_satState(
                        self.handle,
                        state.as_ptr(),
                        fractions.as_mut_ptr(),
                        max_components,
                        &mut actual_len,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })?;

        fractions.truncate(c_long_to_usize("saturated mole fractions", actual_len)?);
        Ok(fractions)
    }

    pub fn fugacity(&mut self, component_index: usize) -> Result<f64> {
        let component_index = usize_to_c_long("component_index", component_index)?;
        with_coolprop(|coolprop| {
            let value = with_error_buffer(
                "AbstractState_get_fugacity",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_get_fugacity(
                        self.handle,
                        component_index,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )?;
            validate_scalar(coolprop, "AbstractState_get_fugacity", value)
        })
    }

    pub fn fugacity_coefficient(&mut self, component_index: usize) -> Result<f64> {
        let component_index = usize_to_c_long("component_index", component_index)?;
        with_coolprop(|coolprop| {
            let value = with_error_buffer(
                "AbstractState_get_fugacity_coefficient",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_get_fugacity_coefficient(
                        self.handle,
                        component_index,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )?;
            validate_scalar(coolprop, "AbstractState_get_fugacity_coefficient", value)
        })
    }

    pub fn set_binary_interaction_double(
        &mut self,
        i: usize,
        j: usize,
        parameter: impl AsRef<str>,
        value: f64,
    ) -> Result<()> {
        let i = usize_to_c_long("i", i)?;
        let j = usize_to_c_long("j", j)?;
        let parameter = c_string(parameter.as_ref(), "parameter")?;

        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_set_binary_interaction_double",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_set_binary_interaction_double(
                        self.handle,
                        i,
                        j,
                        parameter.as_ptr(),
                        value,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })
    }

    pub fn set_cubic_alpha_c(
        &mut self,
        component_index: usize,
        parameter: impl AsRef<str>,
        c1: f64,
        c2: f64,
        c3: f64,
    ) -> Result<()> {
        let component_index = usize_to_c_long("component_index", component_index)?;
        let parameter = c_string(parameter.as_ref(), "parameter")?;

        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_set_cubic_alpha_C",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_set_cubic_alpha_C(
                        self.handle,
                        component_index,
                        parameter.as_ptr(),
                        c1,
                        c2,
                        c3,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })
    }

    pub fn set_fluid_parameter_double(
        &mut self,
        component_index: usize,
        parameter: impl AsRef<str>,
        value: f64,
    ) -> Result<()> {
        let component_index = usize_to_c_long("component_index", component_index)?;
        let parameter = c_string(parameter.as_ref(), "parameter")?;

        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_set_fluid_parameter_double",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_set_fluid_parameter_double(
                        self.handle,
                        component_index,
                        parameter.as_ptr(),
                        value,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })
    }

    pub fn specify_phase(&mut self, phase: PhaseSpecifier) -> Result<()> {
        self.specify_phase_by_name(phase.as_str())
    }

    pub fn specify_phase_by_name(&mut self, phase: impl AsRef<str>) -> Result<()> {
        let phase = c_string(phase.as_ref(), "phase")?;

        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_specify_phase",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_specify_phase(
                        self.handle,
                        phase.as_ptr(),
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })
    }

    pub fn unspecify_phase(&mut self) -> Result<()> {
        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_unspecify_phase",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_unspecify_phase(
                        self.handle,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })
    }

    pub fn phase(&mut self) -> Result<Phase> {
        let phase_index = with_coolprop(|coolprop| {
            let index = with_error_buffer(
                "AbstractState_phase",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_phase(self.handle, errcode, message, message_len)
                },
            )?;
            Ok(index)
        })?;

        match phase_index {
            0 => Ok(Phase::Liquid),
            1 => Ok(Phase::Supercritical),
            2 => Ok(Phase::SupercriticalGas),
            3 => Ok(Phase::SupercriticalLiquid),
            4 => Ok(Phase::CriticalPoint),
            5 => Ok(Phase::Gas),
            6 => Ok(Phase::TwoPhase),
            7 => Ok(Phase::Unknown),
            _ => Ok(Phase::Other(format!("phase_index_{phase_index}"))),
        }
    }

    pub fn first_partial_derivative(
        &mut self,
        of: Parameter,
        with_respect_to: Parameter,
        constant: Parameter,
    ) -> Result<f64> {
        self.first_partial_derivative_by_name(
            of.as_str(),
            with_respect_to.as_str(),
            constant.as_str(),
        )
    }

    pub fn first_partial_derivative_by_name(
        &mut self,
        of: impl AsRef<str>,
        with_respect_to: impl AsRef<str>,
        constant: impl AsRef<str>,
    ) -> Result<f64> {
        let of = of.as_ref();
        let with_respect_to = with_respect_to.as_ref();
        let constant = constant.as_ref();

        with_coolprop(|coolprop| {
            let of = param_index_locked(coolprop, of)?;
            let with_respect_to = param_index_locked(coolprop, with_respect_to)?;
            let constant = param_index_locked(coolprop, constant)?;
            let value = with_error_buffer(
                "AbstractState_first_partial_deriv",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_first_partial_deriv(
                        self.handle,
                        of,
                        with_respect_to,
                        constant,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )?;
            validate_scalar(coolprop, "AbstractState_first_partial_deriv", value)
        })
    }

    pub fn second_partial_derivative(
        &mut self,
        of: Parameter,
        with_respect_to1: Parameter,
        constant1: Parameter,
        with_respect_to2: Parameter,
        constant2: Parameter,
    ) -> Result<f64> {
        self.second_partial_derivative_by_name(
            of.as_str(),
            with_respect_to1.as_str(),
            constant1.as_str(),
            with_respect_to2.as_str(),
            constant2.as_str(),
        )
    }

    pub fn second_partial_derivative_by_name(
        &mut self,
        of: impl AsRef<str>,
        with_respect_to1: impl AsRef<str>,
        constant1: impl AsRef<str>,
        with_respect_to2: impl AsRef<str>,
        constant2: impl AsRef<str>,
    ) -> Result<f64> {
        let of = of.as_ref();
        let with_respect_to1 = with_respect_to1.as_ref();
        let constant1 = constant1.as_ref();
        let with_respect_to2 = with_respect_to2.as_ref();
        let constant2 = constant2.as_ref();

        with_coolprop(|coolprop| {
            let of = param_index_locked(coolprop, of)?;
            let with_respect_to1 = param_index_locked(coolprop, with_respect_to1)?;
            let constant1 = param_index_locked(coolprop, constant1)?;
            let with_respect_to2 = param_index_locked(coolprop, with_respect_to2)?;
            let constant2 = param_index_locked(coolprop, constant2)?;
            let value = with_error_buffer(
                "AbstractState_second_partial_deriv",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_second_partial_deriv(
                        self.handle,
                        of,
                        with_respect_to1,
                        constant1,
                        with_respect_to2,
                        constant2,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )?;
            validate_scalar(coolprop, "AbstractState_second_partial_deriv", value)
        })
    }

    pub fn first_saturation_derivative(
        &mut self,
        of: Parameter,
        with_respect_to: Parameter,
    ) -> Result<f64> {
        self.first_saturation_derivative_by_name(of.as_str(), with_respect_to.as_str())
    }

    pub fn first_saturation_derivative_by_name(
        &mut self,
        of: impl AsRef<str>,
        with_respect_to: impl AsRef<str>,
    ) -> Result<f64> {
        let of = of.as_ref();
        let with_respect_to = with_respect_to.as_ref();

        with_coolprop(|coolprop| {
            let of = param_index_locked(coolprop, of)?;
            let with_respect_to = param_index_locked(coolprop, with_respect_to)?;
            let value = with_error_buffer(
                "AbstractState_first_saturation_deriv",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_first_saturation_deriv(
                        self.handle,
                        of,
                        with_respect_to,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )?;
            validate_scalar(coolprop, "AbstractState_first_saturation_deriv", value)
        })
    }

    pub fn first_two_phase_derivative(
        &mut self,
        of: Parameter,
        with_respect_to: Parameter,
        constant: Parameter,
    ) -> Result<f64> {
        self.first_two_phase_derivative_by_name(
            of.as_str(),
            with_respect_to.as_str(),
            constant.as_str(),
        )
    }

    pub fn first_two_phase_derivative_by_name(
        &mut self,
        of: impl AsRef<str>,
        with_respect_to: impl AsRef<str>,
        constant: impl AsRef<str>,
    ) -> Result<f64> {
        self.two_phase_derivative_impl(
            "AbstractState_first_two_phase_deriv",
            of.as_ref(),
            with_respect_to.as_ref(),
            constant.as_ref(),
            None,
        )
    }

    pub fn first_two_phase_derivative_splined(
        &mut self,
        of: Parameter,
        with_respect_to: Parameter,
        constant: Parameter,
        x_end: f64,
    ) -> Result<f64> {
        self.two_phase_derivative_impl(
            "AbstractState_first_two_phase_deriv_splined",
            of.as_str(),
            with_respect_to.as_str(),
            constant.as_str(),
            Some(x_end),
        )
    }

    pub fn second_two_phase_derivative(
        &mut self,
        of: Parameter,
        with_respect_to1: Parameter,
        constant1: Parameter,
        with_respect_to2: Parameter,
        constant2: Parameter,
    ) -> Result<f64> {
        self.second_two_phase_derivative_by_name(
            of.as_str(),
            with_respect_to1.as_str(),
            constant1.as_str(),
            with_respect_to2.as_str(),
            constant2.as_str(),
        )
    }

    pub fn second_two_phase_derivative_by_name(
        &mut self,
        of: impl AsRef<str>,
        with_respect_to1: impl AsRef<str>,
        constant1: impl AsRef<str>,
        with_respect_to2: impl AsRef<str>,
        constant2: impl AsRef<str>,
    ) -> Result<f64> {
        let of = of.as_ref();
        let with_respect_to1 = with_respect_to1.as_ref();
        let constant1 = constant1.as_ref();
        let with_respect_to2 = with_respect_to2.as_ref();
        let constant2 = constant2.as_ref();

        with_coolprop(|coolprop| {
            let of = param_index_locked(coolprop, of)?;
            let with_respect_to1 = param_index_locked(coolprop, with_respect_to1)?;
            let constant1 = param_index_locked(coolprop, constant1)?;
            let with_respect_to2 = param_index_locked(coolprop, with_respect_to2)?;
            let constant2 = param_index_locked(coolprop, constant2)?;
            let value = with_error_buffer(
                "AbstractState_second_two_phase_deriv",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_second_two_phase_deriv(
                        self.handle,
                        of,
                        with_respect_to1,
                        constant1,
                        with_respect_to2,
                        constant2,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )?;
            validate_scalar(coolprop, "AbstractState_second_two_phase_deriv", value)
        })
    }

    fn two_phase_derivative_impl(
        &mut self,
        function: &'static str,
        of: &str,
        with_respect_to: &str,
        constant: &str,
        x_end: Option<f64>,
    ) -> Result<f64> {
        with_coolprop(|coolprop| {
            let of = param_index_locked(coolprop, of)?;
            let with_respect_to = param_index_locked(coolprop, with_respect_to)?;
            let constant = param_index_locked(coolprop, constant)?;
            let value = with_error_buffer(function, |errcode, message, message_len| unsafe {
                match x_end {
                    Some(x_end) => coolprop.AbstractState_first_two_phase_deriv_splined(
                        self.handle,
                        of,
                        with_respect_to,
                        constant,
                        x_end,
                        errcode,
                        message,
                        message_len,
                    ),
                    None => coolprop.AbstractState_first_two_phase_deriv(
                        self.handle,
                        of,
                        with_respect_to,
                        constant,
                        errcode,
                        message,
                        message_len,
                    ),
                }
            })?;
            validate_scalar(coolprop, function, value)
        })
    }

    pub fn backend_name(&mut self) -> Result<String> {
        let mut buffer = vec![0_u8; 256];
        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_backend_name",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_backend_name(
                        self.handle,
                        buffer.as_mut_ptr().cast::<c_char>(),
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })?;
        Ok(buffer_to_string(&buffer))
    }

    pub fn fluid_names(&mut self) -> Result<Vec<String>> {
        let mut buffer = vec![0_u8; 1_024];
        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_fluid_names",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_fluid_names(
                        self.handle,
                        buffer.as_mut_ptr().cast::<c_char>(),
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })?;

        Ok(buffer_to_string(&buffer)
            .split(',')
            .filter(|name| !name.is_empty())
            .map(ToOwned::to_owned)
            .collect())
    }

    pub fn fluid_param_string(&mut self, param: impl AsRef<str>) -> Result<String> {
        let param = c_string(param.as_ref(), "param")?;
        let mut buffer = vec![0_u8; 4_096];
        let buffer_len = usize_to_c_long("AbstractState_fluid_param_string", buffer.len())?;

        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_fluid_param_string",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_fluid_param_string(
                        self.handle,
                        param.as_ptr(),
                        buffer.as_mut_ptr().cast::<c_char>(),
                        buffer_len,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })?;

        Ok(buffer_to_string(&buffer))
    }

    pub fn saturated_liquid_keyed_output(&mut self, parameter: Parameter) -> Result<f64> {
        self.saturated_liquid_keyed_output_by_name(parameter.as_str())
    }

    pub fn saturated_liquid_keyed_output_by_name(
        &mut self,
        parameter: impl AsRef<str>,
    ) -> Result<f64> {
        let parameter = parameter.as_ref();
        with_coolprop(|coolprop| {
            let parameter = param_index_locked(coolprop, parameter)?;
            let value = with_error_buffer(
                "AbstractState_saturated_liquid_keyed_output",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_saturated_liquid_keyed_output(
                        self.handle,
                        parameter,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )?;
            validate_scalar(
                coolprop,
                "AbstractState_saturated_liquid_keyed_output",
                value,
            )
        })
    }

    pub fn saturated_vapor_keyed_output(&mut self, parameter: Parameter) -> Result<f64> {
        self.saturated_vapor_keyed_output_by_name(parameter.as_str())
    }

    pub fn saturated_vapor_keyed_output_by_name(
        &mut self,
        parameter: impl AsRef<str>,
    ) -> Result<f64> {
        let parameter = parameter.as_ref();
        with_coolprop(|coolprop| {
            let parameter = param_index_locked(coolprop, parameter)?;
            let value = with_error_buffer(
                "AbstractState_saturated_vapor_keyed_output",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_saturated_vapor_keyed_output(
                        self.handle,
                        parameter,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )?;
            validate_scalar(
                coolprop,
                "AbstractState_saturated_vapor_keyed_output",
                value,
            )
        })
    }

    pub fn keyed_output_saturated_state(
        &mut self,
        state: SaturatedState,
        parameter: Parameter,
    ) -> Result<f64> {
        self.keyed_output_saturated_state_by_name(state, parameter.as_str())
    }

    pub fn keyed_output_saturated_state_by_name(
        &mut self,
        state: SaturatedState,
        parameter: impl AsRef<str>,
    ) -> Result<f64> {
        let state = c_string(state.as_str(), "saturated_state")?;
        let parameter = parameter.as_ref();

        with_coolprop(|coolprop| {
            let parameter = param_index_locked(coolprop, parameter)?;
            let value = with_error_buffer(
                "AbstractState_keyed_output_satState",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_keyed_output_satState(
                        self.handle,
                        state.as_ptr(),
                        parameter,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )?;
            validate_scalar(coolprop, "AbstractState_keyed_output_satState", value)
        })
    }

    pub fn build_phase_envelope(&mut self, level: impl AsRef<str>) -> Result<()> {
        let level = c_string(level.as_ref(), "level")?;
        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_build_phase_envelope",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_build_phase_envelope(
                        self.handle,
                        level.as_ptr(),
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })
    }

    pub fn phase_envelope_data(&mut self) -> Result<PhaseEnvelopeData> {
        self.phase_envelope_data_with_capacity(10_000, 20)
    }

    pub fn phase_envelope_data_with_capacity(
        &mut self,
        max_points: usize,
        max_components: usize,
    ) -> Result<PhaseEnvelopeData> {
        let max_points = usize_to_c_long("phase envelope points", max_points)?;
        let max_components = usize_to_c_long("phase envelope components", max_components)?;
        let mut actual_points: c_long = 0;
        let mut actual_components: c_long = 0;
        let mut temperature = vec![0.0; max_points as usize];
        let mut pressure = vec![0.0; max_points as usize];
        let mut vapor_molar_density = vec![0.0; max_points as usize];
        let mut liquid_molar_density = vec![0.0; max_points as usize];
        let composition_len = (max_points as usize)
            .checked_mul(max_components as usize)
            .ok_or(Error::LengthOverflow {
                what: "phase envelope compositions",
                len: usize::MAX,
            })?;
        let mut liquid_mole_fractions = vec![0.0; composition_len];
        let mut vapor_mole_fractions = vec![0.0; composition_len];

        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_get_phase_envelope_data_checkedMemory",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_get_phase_envelope_data_checkedMemory(
                        self.handle,
                        max_points,
                        max_components,
                        temperature.as_mut_ptr(),
                        pressure.as_mut_ptr(),
                        vapor_molar_density.as_mut_ptr(),
                        liquid_molar_density.as_mut_ptr(),
                        liquid_mole_fractions.as_mut_ptr(),
                        vapor_mole_fractions.as_mut_ptr(),
                        &mut actual_points,
                        &mut actual_components,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })?;

        let actual_points = c_long_to_usize("phase envelope points", actual_points)?;
        let actual_components = c_long_to_usize("phase envelope components", actual_components)?;
        let mut points = Vec::with_capacity(actual_points);
        for index in 0..actual_points {
            let start = index * actual_components;
            let end = start + actual_components;
            points.push(PhaseEnvelopePoint {
                temperature: temperature[index],
                pressure: pressure[index],
                vapor_molar_density: vapor_molar_density[index],
                liquid_molar_density: liquid_molar_density[index],
                liquid_mole_fractions: liquid_mole_fractions[start..end].to_vec(),
                vapor_mole_fractions: vapor_mole_fractions[start..end].to_vec(),
            });
        }
        Ok(PhaseEnvelopeData { points })
    }

    pub fn build_spinodal(&mut self) -> Result<()> {
        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_build_spinodal",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_build_spinodal(
                        self.handle,
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })
    }

    pub fn spinodal_data(&mut self) -> Result<Vec<SpinodalPoint>> {
        self.spinodal_data_with_capacity(10_000)
    }

    pub fn spinodal_data_with_capacity(&mut self, max_points: usize) -> Result<Vec<SpinodalPoint>> {
        let max_points = usize_to_c_long("spinodal points", max_points)?;
        let mut tau = vec![0.0; max_points as usize];
        let mut delta = vec![0.0; max_points as usize];
        let mut m1 = vec![0.0; max_points as usize];

        with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_get_spinodal_data",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_get_spinodal_data(
                        self.handle,
                        max_points,
                        tau.as_mut_ptr(),
                        delta.as_mut_ptr(),
                        m1.as_mut_ptr(),
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        })?;

        Ok((0..tau.len())
            .take_while(|index| tau[*index] != 0.0 || delta[*index] != 0.0 || m1[*index] != 0.0)
            .map(|index| SpinodalPoint {
                tau: tau[index],
                delta: delta[index],
                m1: m1[index],
            })
            .collect())
    }

    pub fn all_critical_points(&mut self) -> Result<Vec<CriticalPoint>> {
        self.all_critical_points_with_capacity(64)
    }

    pub fn all_critical_points_with_capacity(
        &mut self,
        max_points: usize,
    ) -> Result<Vec<CriticalPoint>> {
        let max_points = usize_to_c_long("critical points", max_points)?;
        let mut temperature = vec![0.0; max_points as usize];
        let mut pressure = vec![0.0; max_points as usize];
        let mut molar_density = vec![0.0; max_points as usize];
        let mut stable = vec![0; max_points as usize];

        let result = with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_all_critical_points",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_all_critical_points(
                        self.handle,
                        max_points,
                        temperature.as_mut_ptr(),
                        pressure.as_mut_ptr(),
                        molar_density.as_mut_ptr(),
                        stable.as_mut_ptr(),
                        errcode,
                        message,
                        message_len,
                    )
                },
            )
        });

        if let Err(err) = result {
            return self.critical_point().map(|point| vec![point]).or(Err(err));
        }

        Ok((0..temperature.len())
            .take_while(|index| {
                temperature[*index] != 0.0
                    || pressure[*index] != 0.0
                    || molar_density[*index] != 0.0
            })
            .map(|index| CriticalPoint {
                temperature: temperature[index],
                pressure: pressure[index],
                molar_density: molar_density[index],
                stable: stable[index] != 0,
            })
            .collect())
    }

    pub fn critical_point(&mut self) -> Result<CriticalPoint> {
        Ok(CriticalPoint {
            temperature: self.keyed_output_by_name("Tcrit")?,
            pressure: self.keyed_output_by_name("pcrit")?,
            molar_density: self.keyed_output_by_name("rhomolar_critical")?,
            stable: true,
        })
    }
}

impl Drop for AbstractState {
    fn drop(&mut self) {
        let _ = with_coolprop(|coolprop| {
            with_error_buffer(
                "AbstractState_free",
                |errcode, message, message_len| unsafe {
                    coolprop.AbstractState_free(self.handle, errcode, message, message_len)
                },
            )
        });
    }
}

fn ensure_same_len(
    left: &'static str,
    left_len: usize,
    right: &'static str,
    right_len: usize,
) -> Result<()> {
    if left_len == right_len {
        Ok(())
    } else {
        Err(Error::DimensionMismatch {
            left,
            left_len,
            right,
            right_len,
        })
    }
}

fn usize_to_c_long(what: &'static str, len: usize) -> Result<c_long> {
    len.try_into()
        .map_err(|_| Error::LengthOverflow { what, len })
}

fn c_long_to_usize(what: &'static str, value: c_long) -> Result<usize> {
    value.try_into().map_err(|_| Error::LengthOverflow {
        what,
        len: value.unsigned_abs() as usize,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abstract_state_updates_and_outputs() {
        let mut state = AbstractState::new("HEOS", "Water").unwrap();
        state
            .update(InputPair::PressureTemperature, 101_325.0, 300.0)
            .unwrap();

        let cp = state
            .keyed_output(Parameter::MassSpecificHeatConstantPressure)
            .unwrap();
        assert!((cp - 4_180.635_776_556_071_5).abs() < 1.0e-6);
    }

    #[test]
    fn abstract_state_can_move_to_thread() {
        let mut state = AbstractState::new("HEOS", "Water").unwrap();
        let thread = std::thread::spawn(move || {
            state
                .update(InputPair::PressureTemperature, 101_325.0, 300.0)
                .unwrap();
            state.mass_density().unwrap()
        });

        assert!(thread.join().unwrap() > 990.0);
    }

    #[test]
    fn common_outputs_batch() {
        let mut state = AbstractState::new("HEOS", "Water").unwrap();
        let outputs = state
            .common_outputs(
                InputPair::PressureTemperature,
                &[101_325.0, 101_325.0],
                &[300.0, 310.0],
            )
            .unwrap();

        assert_eq!(outputs.len(), 2);
        assert!((outputs[0].pressure - 101_325.0).abs() < 1.0e-2);
        assert!(outputs[1].temperature > outputs[0].temperature);
    }

    #[test]
    fn typed_update_and_batch_output_work() {
        let mut state = AbstractState::new("HEOS", "Water").unwrap();
        state
            .update_state(StateUpdate::pressure_temperature(101_325.0, 300.0))
            .unwrap();
        assert!(state.viscosity().unwrap() > 0.0);

        let density = state
            .update_and_output(
                InputPair::PressureTemperature,
                &[101_325.0, 101_325.0],
                &[300.0, 310.0],
                Parameter::MassDensity,
            )
            .unwrap();
        assert_eq!(density.len(), 2);
        assert!(density[0] > density[1]);
    }

    #[test]
    fn saturated_outputs_and_derivatives_work() {
        let mut state = AbstractState::new("HEOS", "Water").unwrap();
        state
            .update(InputPair::PressureQuality, 101_325.0, 0.0)
            .unwrap();

        let liquid_density = state
            .keyed_output_saturated_state(SaturatedState::Liquid, Parameter::MassDensity)
            .unwrap();
        let vapor_density = state
            .keyed_output_saturated_state(SaturatedState::Vapor, Parameter::MassDensity)
            .unwrap();
        assert!(liquid_density > vapor_density);

        let dpdt = state
            .first_saturation_derivative(Parameter::Pressure, Parameter::Temperature)
            .unwrap();
        assert!(dpdt > 0.0);
    }

    #[test]
    fn composition_and_metadata_work() {
        let mut state = AbstractState::new("HEOS", "Water").unwrap();
        state.set_fractions(&[1.0]).unwrap();
        assert_eq!(state.mole_fractions().unwrap(), vec![1.0]);
        assert_eq!(state.fluid_names().unwrap(), vec!["Water".to_owned()]);
        assert!(state.fluid_param_string("CAS").unwrap().contains("7732"));
    }

    #[test]
    fn critical_points_work() {
        let mut state = AbstractState::new("HEOS", "Water").unwrap();
        let points = state.all_critical_points().unwrap();
        assert!(!points.is_empty());
        assert!(points[0].temperature > 600.0);
    }

    #[test]
    fn phase_envelope_and_spinodal_work() {
        let mut state = AbstractState::new("HEOS", "Water").unwrap();
        state.build_phase_envelope("").unwrap();
        let envelope = state.phase_envelope_data_with_capacity(512, 1).unwrap();
        assert!(!envelope.points.is_empty());

        state.build_spinodal().unwrap();
        let spinodal = state.spinodal_data_with_capacity(512).unwrap();
        assert!(!spinodal.is_empty());
    }
}
