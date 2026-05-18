use std::cell::Cell;
use std::marker::PhantomData;
use std::os::raw::{c_char, c_long};

use crate::{
    buffer_to_string, c_string, input_pair_index_locked, param_index_locked, validate_scalar,
    with_coolprop, with_error_buffer, Error, InputPair, Parameter, Phase, PhaseSpecifier, Result,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommonOutputs {
    pub temperature: f64,
    pub pressure: f64,
    pub molar_density: f64,
    pub molar_enthalpy: f64,
    pub molar_entropy: f64,
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
}
