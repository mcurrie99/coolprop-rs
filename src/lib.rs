//! Safe Rust interface for CoolProp.
//!
//! The crate mirrors the public CoolProp interface shape:
//! high-level `PropsSI`/`PhaseSI` calls for quick use, and an
//! `AbstractState` wrapper for repeated state updates.

mod error;
mod keys;
mod multi;
mod state;

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_long};
use std::sync::LazyLock;
#[cfg(not(feature = "unsafe-reentrant"))]
use std::sync::{Mutex, MutexGuard};

use coolprop_sys::bindings::CoolProp;

pub use crate::error::{Error, Result};
pub use crate::keys::{
    InputPair, Parameter, Phase, PhaseSpecifier, ReferenceState, SaturatedState, SaturationBranch,
};
pub use crate::multi::{props1_si_multi, props_si_multi, props_si_multi_pure, PropsSIOutput};
pub use crate::state::{
    AbstractState, BatchFiveOutputs, CommonOutputs, CriticalPoint, PhaseEnvelopeData,
    PhaseEnvelopePoint, SpinodalPoint, StateUpdate,
};

const STRING_BUFFER_LEN: usize = 65_536;
const ERROR_BUFFER_LEN: usize = 4_096;
const COOLPROP_ERROR_SENTINEL_ABS: f64 = 1.0e90;

#[cfg(not(feature = "unsafe-reentrant"))]
static COOLPROP: LazyLock<Result<Mutex<CoolProp>>> = LazyLock::new(|| {
    let path = coolprop_sys::COOLPROP_PATH;
    let library = unsafe { CoolProp::new(path) }.map_err(|err| Error::LibraryLoad {
        path: path.to_owned(),
        message: err.to_string(),
    })?;
    Ok(Mutex::new(library))
});

#[cfg(feature = "unsafe-reentrant")]
static COOLPROP: LazyLock<Result<ReentrantCoolProp>> = LazyLock::new(|| {
    let path = coolprop_sys::COOLPROP_PATH;
    let library = unsafe { CoolProp::new(path) }.map_err(|err| Error::LibraryLoad {
        path: path.to_owned(),
        message: err.to_string(),
    })?;
    preload_coolprop_library(&library)?;
    Ok(ReentrantCoolProp(library))
});

#[cfg(feature = "unsafe-reentrant")]
struct ReentrantCoolProp(CoolProp);

#[cfg(feature = "unsafe-reentrant")]
unsafe impl Send for ReentrantCoolProp {}

#[cfg(feature = "unsafe-reentrant")]
unsafe impl Sync for ReentrantCoolProp {}

#[cfg(feature = "unsafe-reentrant")]
fn preload_coolprop_library(library: &CoolProp) -> Result<()> {
    let param = CString::new("FluidsList").expect("static string does not contain NUL");
    let mut buffer = vec![0_u8; STRING_BUFFER_LEN];
    let status = unsafe {
        library.get_global_param_string(
            param.as_ptr(),
            buffer.as_mut_ptr().cast::<c_char>(),
            STRING_BUFFER_LEN as c_int,
        )
    };
    if status == 1 {
        Ok(())
    } else {
        Err(Error::coolprop_message(
            "failed to preload CoolProp fluid library",
        ))
    }
}

pub fn props_si(
    output: impl AsRef<str>,
    name1: impl AsRef<str>,
    prop1: f64,
    name2: impl AsRef<str>,
    prop2: f64,
    fluid_name: impl AsRef<str>,
) -> Result<f64> {
    let output = c_string(output.as_ref(), "output")?;
    let name1 = c_string(name1.as_ref(), "name1")?;
    let name2 = c_string(name2.as_ref(), "name2")?;
    let fluid_name = c_string(fluid_name.as_ref(), "fluid_name")?;

    with_coolprop(|coolprop| {
        let value = unsafe {
            coolprop.PropsSI(
                output.as_ptr(),
                name1.as_ptr(),
                prop1,
                name2.as_ptr(),
                prop2,
                fluid_name.as_ptr(),
            )
        };
        validate_scalar(coolprop, "PropsSI", value)
    })
}

pub fn props1_si(fluid_name: impl AsRef<str>, output: impl AsRef<str>) -> Result<f64> {
    let fluid_name = c_string(fluid_name.as_ref(), "fluid_name")?;
    let output = c_string(output.as_ref(), "output")?;

    with_coolprop(|coolprop| {
        let value = unsafe { coolprop.Props1SI(fluid_name.as_ptr(), output.as_ptr()) };
        validate_scalar(coolprop, "Props1SI", value)
    })
}

pub fn phase_si(
    name1: impl AsRef<str>,
    prop1: f64,
    name2: impl AsRef<str>,
    prop2: f64,
    fluid_name: impl AsRef<str>,
) -> Result<Phase> {
    phase_si_raw(name1, prop1, name2, prop2, fluid_name).map(Phase::from_coolprop)
}

pub fn phase_si_raw(
    name1: impl AsRef<str>,
    prop1: f64,
    name2: impl AsRef<str>,
    prop2: f64,
    fluid_name: impl AsRef<str>,
) -> Result<String> {
    let name1 = c_string(name1.as_ref(), "name1")?;
    let name2 = c_string(name2.as_ref(), "name2")?;
    let fluid_name = c_string(fluid_name.as_ref(), "fluid_name")?;

    with_coolprop(|coolprop| {
        let mut buffer = vec![0_u8; 256];
        let status = unsafe {
            coolprop.PhaseSI(
                name1.as_ptr(),
                prop1,
                name2.as_ptr(),
                prop2,
                fluid_name.as_ptr(),
                buffer.as_mut_ptr().cast::<c_char>(),
                buffer_len_to_c_int("PhaseSI", buffer.len())?,
            )
        };

        if status == 1 {
            Ok(buffer_to_string(&buffer))
        } else {
            Err(last_error(coolprop, "PhaseSI"))
        }
    })
}

pub fn ha_props_si(
    output: impl AsRef<str>,
    name1: impl AsRef<str>,
    prop1: f64,
    name2: impl AsRef<str>,
    prop2: f64,
    name3: impl AsRef<str>,
    prop3: f64,
) -> Result<f64> {
    let output = c_string(output.as_ref(), "output")?;
    let name1 = c_string(name1.as_ref(), "name1")?;
    let name2 = c_string(name2.as_ref(), "name2")?;
    let name3 = c_string(name3.as_ref(), "name3")?;

    with_coolprop(|coolprop| {
        let value = unsafe {
            coolprop.HAPropsSI(
                output.as_ptr(),
                name1.as_ptr(),
                prop1,
                name2.as_ptr(),
                prop2,
                name3.as_ptr(),
                prop3,
            )
        };
        validate_scalar(coolprop, "HAPropsSI", value)
    })
}

pub fn saturation_ancillary(
    fluid_name: impl AsRef<str>,
    output: impl AsRef<str>,
    branch: SaturationBranch,
    input: impl AsRef<str>,
    value: f64,
) -> Result<f64> {
    let fluid_name = c_string(fluid_name.as_ref(), "fluid_name")?;
    let output = c_string(output.as_ref(), "output")?;
    let input = c_string(input.as_ref(), "input")?;

    with_coolprop(|coolprop| {
        let value = unsafe {
            coolprop.saturation_ancillary(
                fluid_name.as_ptr(),
                output.as_ptr(),
                branch.as_quality(),
                input.as_ptr(),
                value,
            )
        };
        validate_scalar(coolprop, "saturation_ancillary", value)
    })
}

pub fn fahrenheit_to_kelvin(temperature_f: f64) -> Result<f64> {
    with_coolprop(|coolprop| Ok(unsafe { coolprop.F2K(temperature_f) }))
}

pub fn kelvin_to_fahrenheit(temperature_k: f64) -> Result<f64> {
    with_coolprop(|coolprop| Ok(unsafe { coolprop.K2F(temperature_k) }))
}

pub fn global_param_string(param: impl AsRef<str>) -> Result<String> {
    let param = c_string(param.as_ref(), "param")?;
    with_coolprop(|coolprop| global_param_string_locked(coolprop, &param, STRING_BUFFER_LEN))
}

pub fn version() -> Result<String> {
    global_param_string("version")
}

pub fn git_revision() -> Result<String> {
    global_param_string("gitrevision")
}

pub fn fluids_list() -> Result<String> {
    global_param_string("FluidsList")
}

pub fn predefined_mixtures_list() -> Result<String> {
    global_param_string("predefined_mixtures")
}

pub fn parameter_information_string(
    parameter: impl AsRef<str>,
    information: impl AsRef<str>,
) -> Result<String> {
    let parameter = c_string(parameter.as_ref(), "parameter")?;
    let information = c_string(information.as_ref(), "information")?;

    with_coolprop(|coolprop| {
        let mut buffer = vec![0_u8; STRING_BUFFER_LEN];
        let info_bytes = information.as_bytes_with_nul();
        if info_bytes.len() > buffer.len() {
            return Err(Error::BufferTooSmall {
                function: "get_parameter_information_string",
                size: buffer.len(),
            });
        }
        buffer[..info_bytes.len()].copy_from_slice(info_bytes);

        let status = unsafe {
            coolprop.get_parameter_information_string(
                parameter.as_ptr(),
                buffer.as_mut_ptr().cast::<c_char>(),
                buffer_len_to_c_int("get_parameter_information_string", buffer.len())?,
            )
        };

        if status == 1 {
            Ok(buffer_to_string(&buffer))
        } else {
            let info = information.to_string_lossy();
            Err(Error::coolprop_message(format!(
                "unable to read parameter information field {info}"
            )))
        }
    })
}

pub fn parameter_index(param: impl AsRef<str>) -> Result<c_long> {
    let name = param.as_ref();
    let param = c_string(name, "param")?;

    with_coolprop(|coolprop| {
        let index = unsafe { coolprop.get_param_index(param.as_ptr()) };
        if index >= 0 {
            Ok(index)
        } else {
            Err(invalid_key(coolprop, "parameter", name))
        }
    })
}

pub fn input_pair_index(pair: impl AsRef<str>) -> Result<c_long> {
    let name = pair.as_ref();
    let pair = c_string(name, "pair")?;

    with_coolprop(|coolprop| {
        let index = unsafe { coolprop.get_input_pair_index(pair.as_ptr()) };
        if index >= 0 {
            Ok(index)
        } else {
            Err(invalid_key(coolprop, "input pair", name))
        }
    })
}

pub fn fluid_param_string(fluid: impl AsRef<str>, param: impl AsRef<str>) -> Result<String> {
    let fluid = c_string(fluid.as_ref(), "fluid")?;
    let param = c_string(param.as_ref(), "param")?;

    with_coolprop(|coolprop| {
        let len = unsafe { coolprop.get_fluid_param_string_len(fluid.as_ptr(), param.as_ptr()) };
        if len < 0 {
            return Err(last_error(coolprop, "get_fluid_param_string_len"));
        }

        let mut buffer = vec![0_u8; len as usize + 1];
        let status = unsafe {
            coolprop.get_fluid_param_string(
                fluid.as_ptr(),
                param.as_ptr(),
                buffer.as_mut_ptr().cast::<c_char>(),
                buffer_len_to_c_int("get_fluid_param_string", buffer.len())?,
            )
        };

        if status == 1 {
            Ok(buffer_to_string(&buffer))
        } else {
            Err(last_error(coolprop, "get_fluid_param_string"))
        }
    })
}

pub fn is_valid_fluid_string(fluid: impl AsRef<str>) -> Result<bool> {
    let fluid = c_string(fluid.as_ref(), "fluid")?;
    with_coolprop(|coolprop| {
        let status = unsafe { coolprop.C_is_valid_fluid_string(fluid.as_ptr()) };
        Ok(status != 0)
    })
}

pub fn set_reference_state(fluid: impl AsRef<str>, reference_state: ReferenceState) -> Result<()> {
    set_reference_state_by_name(fluid, reference_state.as_str())
}

pub fn set_reference_state_by_name(
    fluid: impl AsRef<str>,
    reference_state: impl AsRef<str>,
) -> Result<()> {
    let fluid = c_string(fluid.as_ref(), "fluid")?;
    let reference_state = c_string(reference_state.as_ref(), "reference_state")?;

    with_coolprop(|coolprop| {
        let status =
            unsafe { coolprop.set_reference_stateS(fluid.as_ptr(), reference_state.as_ptr()) };
        if status == 1 {
            Ok(())
        } else {
            Err(last_error(coolprop, "set_reference_stateS"))
        }
    })
}

pub fn set_reference_state_custom(
    fluid: impl AsRef<str>,
    temperature: f64,
    molar_density: f64,
    molar_enthalpy0: f64,
    molar_entropy0: f64,
) -> Result<()> {
    let fluid = c_string(fluid.as_ref(), "fluid")?;

    with_coolprop(|coolprop| {
        let status = unsafe {
            coolprop.set_reference_stateD(
                fluid.as_ptr(),
                temperature,
                molar_density,
                molar_enthalpy0,
                molar_entropy0,
            )
        };
        if status == 1 {
            Ok(())
        } else {
            Err(last_error(coolprop, "set_reference_stateD"))
        }
    })
}

pub fn add_fluids_as_json(backend: impl AsRef<str>, fluid_json: impl AsRef<str>) -> Result<()> {
    let backend = c_string(backend.as_ref(), "backend")?;
    let fluid_json = c_string(fluid_json.as_ref(), "fluid_json")?;

    with_coolprop(|coolprop| {
        with_error_buffer(
            "add_fluids_as_JSON",
            |errcode, message, message_len| unsafe {
                coolprop.add_fluids_as_JSON(
                    backend.as_ptr(),
                    fluid_json.as_ptr(),
                    errcode,
                    message,
                    message_len,
                )
            },
        )
    })
}

pub fn extract_backend(fluid_string: impl AsRef<str>) -> Result<(String, String)> {
    let fluid_string = c_string(fluid_string.as_ref(), "fluid_string")?;

    with_coolprop(|coolprop| {
        let mut backend = vec![0_u8; 256];
        let mut fluid = vec![0_u8; STRING_BUFFER_LEN];
        let status = unsafe {
            coolprop.C_extract_backend(
                fluid_string.as_ptr(),
                backend.as_mut_ptr().cast::<c_char>(),
                buffer_len_to_c_long("C_extract_backend backend", backend.len())?,
                fluid.as_mut_ptr().cast::<c_char>(),
                buffer_len_to_c_long("C_extract_backend fluid", fluid.len())?,
            )
        };
        if status == 0 {
            Ok((buffer_to_string(&backend), buffer_to_string(&fluid)))
        } else {
            Err(last_error(coolprop, "C_extract_backend"))
        }
    })
}

pub fn debug_level() -> Result<c_int> {
    with_coolprop(|coolprop| Ok(unsafe { coolprop.get_debug_level() }))
}

pub fn set_debug_level(level: c_int) -> Result<()> {
    with_coolprop(|coolprop| {
        unsafe { coolprop.set_debug_level(level) };
        Ok(())
    })
}

pub fn set_config_string(key: impl AsRef<str>, value: impl AsRef<str>) -> Result<()> {
    let key = c_string(key.as_ref(), "key")?;
    let value = c_string(value.as_ref(), "value")?;
    with_coolprop(|coolprop| {
        unsafe { coolprop.set_config_string(key.as_ptr(), value.as_ptr()) };
        Ok(())
    })
}

pub fn set_config_double(key: impl AsRef<str>, value: f64) -> Result<()> {
    let key = c_string(key.as_ref(), "key")?;
    with_coolprop(|coolprop| {
        unsafe { coolprop.set_config_double(key.as_ptr(), value) };
        Ok(())
    })
}

pub fn set_config_bool(key: impl AsRef<str>, value: bool) -> Result<()> {
    let key = c_string(key.as_ref(), "key")?;
    with_coolprop(|coolprop| {
        unsafe { coolprop.set_config_bool(key.as_ptr(), value) };
        Ok(())
    })
}

#[allow(non_snake_case)]
pub fn PropsSI(
    output: impl AsRef<str>,
    name1: impl AsRef<str>,
    prop1: f64,
    name2: impl AsRef<str>,
    prop2: f64,
    fluid_name: impl AsRef<str>,
) -> Result<f64> {
    props_si(output, name1, prop1, name2, prop2, fluid_name)
}

#[allow(non_snake_case)]
pub fn Props1SI(fluid_name: impl AsRef<str>, output: impl AsRef<str>) -> Result<f64> {
    props1_si(fluid_name, output)
}

#[allow(non_snake_case)]
pub fn PhaseSI(
    name1: impl AsRef<str>,
    prop1: f64,
    name2: impl AsRef<str>,
    prop2: f64,
    fluid_name: impl AsRef<str>,
) -> Result<Phase> {
    phase_si(name1, prop1, name2, prop2, fluid_name)
}

#[allow(non_snake_case)]
pub fn HAPropsSI(
    output: impl AsRef<str>,
    name1: impl AsRef<str>,
    prop1: f64,
    name2: impl AsRef<str>,
    prop2: f64,
    name3: impl AsRef<str>,
    prop3: f64,
) -> Result<f64> {
    ha_props_si(output, name1, prop1, name2, prop2, name3, prop3)
}

#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
pub fn PropsSImulti(
    outputs: &[impl AsRef<str>],
    name1: impl AsRef<str>,
    prop1: &[f64],
    name2: impl AsRef<str>,
    prop2: &[f64],
    backend: impl AsRef<str>,
    fluids: &[impl AsRef<str>],
    fractions: &[f64],
) -> Result<PropsSIOutput> {
    props_si_multi(
        outputs, name1, prop1, name2, prop2, backend, fluids, fractions,
    )
}

#[allow(non_snake_case)]
pub fn Props1SImulti(
    outputs: &[impl AsRef<str>],
    backend: impl AsRef<str>,
    fluids: &[impl AsRef<str>],
    fractions: &[f64],
) -> Result<Vec<f64>> {
    props1_si_multi(outputs, backend, fluids, fractions)
}

#[allow(non_snake_case)]
pub fn SaturationAncillary(
    fluid_name: impl AsRef<str>,
    output: impl AsRef<str>,
    branch: SaturationBranch,
    input: impl AsRef<str>,
    value: f64,
) -> Result<f64> {
    saturation_ancillary(fluid_name, output, branch, input, value)
}

#[cfg(not(feature = "unsafe-reentrant"))]
pub(crate) fn with_coolprop<T>(f: impl FnOnce(&CoolProp) -> Result<T>) -> Result<T> {
    let guard = coolprop_guard()?;
    f(&guard)
}

#[cfg(feature = "unsafe-reentrant")]
pub(crate) fn with_coolprop<T>(f: impl FnOnce(&CoolProp) -> Result<T>) -> Result<T> {
    match &*COOLPROP {
        Ok(library) => f(&library.0),
        Err(err) => Err(err.clone()),
    }
}

pub(crate) fn c_string(value: &str, field: &'static str) -> Result<CString> {
    CString::new(value).map_err(|_| Error::NulByte {
        field,
        value: value.to_owned(),
    })
}

pub(crate) fn with_error_buffer<T>(
    function: &'static str,
    f: impl FnOnce(*mut c_long, *mut c_char, c_long) -> T,
) -> Result<T> {
    let mut code: c_long = 0;
    let mut buffer = vec![0_u8; ERROR_BUFFER_LEN];
    let value = f(
        &mut code,
        buffer.as_mut_ptr().cast::<c_char>(),
        buffer_len_to_c_long(function, buffer.len())?,
    );

    if code == 0 {
        Ok(value)
    } else {
        let message = buffer_to_string(&buffer);
        Err(Error::coolprop_code(code as i64, message))
    }
}

pub(crate) fn param_index_locked(coolprop: &CoolProp, name: &str) -> Result<c_long> {
    let key = c_string(name, "parameter")?;
    let index = unsafe { coolprop.get_param_index(key.as_ptr()) };
    if index >= 0 {
        Ok(index)
    } else {
        Err(invalid_key(coolprop, "parameter", name))
    }
}

pub(crate) fn input_pair_index_locked(coolprop: &CoolProp, name: &str) -> Result<c_long> {
    let key = c_string(name, "input_pair")?;
    let index = unsafe { coolprop.get_input_pair_index(key.as_ptr()) };
    if index >= 0 {
        Ok(index)
    } else {
        Err(invalid_key(coolprop, "input pair", name))
    }
}

pub(crate) fn validate_scalar(
    coolprop: &CoolProp,
    function: &'static str,
    value: f64,
) -> Result<f64> {
    if value.is_finite() && value.abs() < COOLPROP_ERROR_SENTINEL_ABS {
        Ok(value)
    } else {
        Err(last_error(coolprop, function))
    }
}

pub(crate) fn buffer_to_string(buffer: &[u8]) -> String {
    let len = buffer
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(buffer.len());
    String::from_utf8_lossy(&buffer[..len]).trim().to_owned()
}

#[cfg(not(feature = "unsafe-reentrant"))]
fn coolprop_guard() -> Result<MutexGuard<'static, CoolProp>> {
    match &*COOLPROP {
        Ok(mutex) => mutex.lock().map_err(|_| Error::LockPoisoned),
        Err(err) => Err(err.clone()),
    }
}

fn global_param_string_locked(
    coolprop: &CoolProp,
    param: &CString,
    buffer_len: usize,
) -> Result<String> {
    let mut buffer = vec![0_u8; buffer_len];
    let status = unsafe {
        coolprop.get_global_param_string(
            param.as_ptr(),
            buffer.as_mut_ptr().cast::<c_char>(),
            buffer_len_to_c_int("get_global_param_string", buffer.len())?,
        )
    };

    if status == 1 {
        let output = buffer_to_string(&buffer);
        if output.is_empty() {
            Err(Error::coolprop_message("empty CoolProp response"))
        } else {
            Ok(output)
        }
    } else {
        Err(Error::coolprop_message(format!(
            "unable to read global parameter {:?}",
            param
        )))
    }
}

pub(crate) fn last_error(coolprop: &CoolProp, function: &'static str) -> Error {
    let message = c_string("errstring", "param")
        .and_then(|param| global_param_string_locked(coolprop, &param, STRING_BUFFER_LEN))
        .unwrap_or_else(|_| "CoolProp did not provide an error string".to_owned());

    Error::InvalidOutput { function, message }
}

fn invalid_key(coolprop: &CoolProp, kind: &'static str, name: &str) -> Error {
    let message = c_string("errstring", "param")
        .and_then(|param| global_param_string_locked(coolprop, &param, STRING_BUFFER_LEN))
        .unwrap_or_default();

    if message.is_empty() {
        Error::InvalidKey {
            kind,
            name: name.to_owned(),
        }
    } else {
        Error::CoolProp {
            code: None,
            message,
        }
    }
}

fn buffer_len_to_c_int(function: &'static str, len: usize) -> Result<c_int> {
    len.try_into().map_err(|_| Error::LengthOverflow {
        what: function,
        len,
    })
}

pub(crate) fn buffer_len_to_c_long(function: &'static str, len: usize) -> Result<c_long> {
    len.try_into().map_err(|_| Error::LengthOverflow {
        what: function,
        len,
    })
}

pub(crate) fn usize_to_c_long(what: &'static str, len: usize) -> Result<c_long> {
    len.try_into()
        .map_err(|_| Error::LengthOverflow { what, len })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn props_si_matches_coolprop_boiling_point_example() {
        let temperature = PropsSI("T", "P", 101_325.0, "Q", 0.0, "Water").unwrap();
        assert!((temperature - 373.124_295_847_666_5).abs() < 1.0e-6);
    }

    #[test]
    fn phase_si_returns_typed_phase() {
        let phase = PhaseSI("P", 101_325.0, "T", 300.0, "Water").unwrap();
        assert_eq!(phase, Phase::Liquid);
    }

    #[test]
    fn invalid_props_si_returns_error() {
        let err = props_si("not-a-property", "P", 101_325.0, "Q", 0.0, "Water").unwrap_err();
        assert!(err.to_string().contains("PropsSI"));
    }

    #[test]
    fn threaded_high_level_calls_are_safe() {
        let mut threads = Vec::new();
        for _ in 0..4 {
            threads.push(std::thread::spawn(|| {
                for _ in 0..10 {
                    let density = props_si("Dmass", "T", 300.0, "P", 101_325.0, "Water").unwrap();
                    assert!(density > 990.0);
                }
            }));
        }

        for thread in threads {
            thread.join().unwrap();
        }
    }

    #[test]
    fn metadata_helpers_work() {
        assert!(!version().unwrap().is_empty());
        assert_eq!(parameter_information_string("T", "units").unwrap(), "K");
        assert!(
            saturation_ancillary("Water", "P", SaturationBranch::Liquid, "T", 300.0).unwrap() > 0.0
        );
        assert_eq!(
            extract_backend("HEOS::Water").unwrap(),
            ("HEOS".to_owned(), "Water".to_owned())
        );
    }
}
