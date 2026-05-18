use std::ffi::CString;
use std::os::raw::{c_char, c_long};

use crate::{c_string, last_error, usize_to_c_long, validate_scalar, with_coolprop, Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct PropsSIOutput {
    rows: Vec<Vec<f64>>,
}

impl PropsSIOutput {
    pub fn new(rows: Vec<Vec<f64>>) -> Self {
        Self { rows }
    }

    pub fn rows(&self) -> &[Vec<f64>] {
        &self.rows
    }

    pub fn into_rows(self) -> Vec<Vec<f64>> {
        self.rows
    }

    pub fn get(&self, state_index: usize, output_index: usize) -> Option<f64> {
        self.rows
            .get(state_index)
            .and_then(|row| row.get(output_index))
            .copied()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn props_si_multi(
    outputs: &[impl AsRef<str>],
    name1: impl AsRef<str>,
    prop1: &[f64],
    name2: impl AsRef<str>,
    prop2: &[f64],
    backend: impl AsRef<str>,
    fluids: &[impl AsRef<str>],
    fractions: &[f64],
) -> Result<PropsSIOutput> {
    if prop1.len() != prop2.len() {
        return Err(Error::DimensionMismatch {
            left: "prop1",
            left_len: prop1.len(),
            right: "prop2",
            right_len: prop2.len(),
        });
    }
    if fluids.len() != fractions.len() {
        return Err(Error::DimensionMismatch {
            left: "fluids",
            left_len: fluids.len(),
            right: "fractions",
            right_len: fractions.len(),
        });
    }

    let outputs_len = outputs.len();
    let states_len = prop1.len();
    if outputs_len == 0 {
        return Err(Error::EmptyInput { what: "outputs" });
    }
    if states_len == 0 {
        return Err(Error::EmptyInput {
            what: "input properties",
        });
    }
    if fluids.is_empty() {
        return Err(Error::EmptyInput { what: "fluids" });
    }

    let outputs = join_coolprop_list(outputs, "outputs")?;
    let name1 = c_string(name1.as_ref(), "name1")?;
    let mut prop1 = prop1.to_vec();
    let name2 = c_string(name2.as_ref(), "name2")?;
    let mut prop2 = prop2.to_vec();
    let mut backend = c_string(backend.as_ref(), "backend")?.into_bytes_with_nul();
    let fluids = join_coolprop_list(fluids, "fluids")?;
    let fractions_len = usize_to_c_long("fractions", fractions.len())?;
    let mut resdim1 = usize_to_c_long("outputs", outputs_len)?;
    let mut resdim2 = usize_to_c_long("states", states_len)?;
    let total_len = outputs_len
        .checked_mul(states_len)
        .ok_or(Error::LengthOverflow {
            what: "PropsSImulti result",
            len: usize::MAX,
        })?;
    let mut result = vec![0.0; total_len];

    with_coolprop(|coolprop| {
        unsafe {
            coolprop.PropsSImulti(
                outputs.as_ptr(),
                name1.as_ptr(),
                prop1.as_mut_ptr(),
                usize_to_c_long("prop1", prop1.len())?,
                name2.as_ptr(),
                prop2.as_mut_ptr(),
                usize_to_c_long("prop2", prop2.len())?,
                backend.as_mut_ptr().cast::<c_char>(),
                fluids.as_ptr(),
                fractions.as_ptr(),
                fractions_len,
                result.as_mut_ptr(),
                &mut resdim1,
                &mut resdim2,
            );
        }

        if resdim1 == 0 || resdim2 == 0 {
            return Err(last_error(coolprop, "PropsSImulti"));
        }

        let rows = c_long_to_usize("PropsSImulti rows", resdim1)?;
        let cols = c_long_to_usize("PropsSImulti columns", resdim2)?;
        if rows > outputs_len || cols > states_len {
            return Err(Error::coolprop_message(format!(
                "PropsSImulti returned dimensions {rows} x {cols}, larger than allocated {outputs_len} x {states_len}"
            )));
        }

        let mut matrix = Vec::with_capacity(rows);
        for row in 0..rows {
            let mut values = Vec::with_capacity(cols);
            for col in 0..cols {
                let value = result[row * cols + col];
                values.push(validate_scalar(coolprop, "PropsSImulti", value)?);
            }
            matrix.push(values);
        }
        Ok(PropsSIOutput::new(matrix))
    })
}

pub fn props_si_multi_pure(
    outputs: &[impl AsRef<str>],
    name1: impl AsRef<str>,
    prop1: &[f64],
    name2: impl AsRef<str>,
    prop2: &[f64],
    fluid: impl AsRef<str>,
) -> Result<PropsSIOutput> {
    props_si_multi(
        outputs,
        name1,
        prop1,
        name2,
        prop2,
        "",
        &[fluid.as_ref()],
        &[1.0],
    )
}

pub fn props1_si_multi(
    outputs: &[impl AsRef<str>],
    backend: impl AsRef<str>,
    fluids: &[impl AsRef<str>],
    fractions: &[f64],
) -> Result<Vec<f64>> {
    if outputs.is_empty() {
        return Err(Error::EmptyInput { what: "outputs" });
    }
    if fluids.len() != fractions.len() {
        return Err(Error::DimensionMismatch {
            left: "fluids",
            left_len: fluids.len(),
            right: "fractions",
            right_len: fractions.len(),
        });
    }
    if fluids.is_empty() {
        return Err(Error::EmptyInput { what: "fluids" });
    }

    let outputs_len = outputs.len();
    let outputs = join_coolprop_list(outputs, "outputs")?;
    let mut backend = c_string(backend.as_ref(), "backend")?.into_bytes_with_nul();
    let fluids = join_coolprop_list(fluids, "fluids")?;
    let mut resdim1 = usize_to_c_long("outputs", outputs_len)?;
    let mut result = vec![0.0; outputs_len];

    with_coolprop(|coolprop| {
        unsafe {
            coolprop.Props1SImulti(
                outputs.as_ptr(),
                backend.as_mut_ptr().cast::<c_char>(),
                fluids.as_ptr(),
                fractions.as_ptr(),
                usize_to_c_long("fractions", fractions.len())?,
                result.as_mut_ptr(),
                &mut resdim1,
            );
        }

        if resdim1 == 0 {
            return Err(last_error(coolprop, "Props1SImulti"));
        }

        let len = c_long_to_usize("Props1SImulti result", resdim1)?;
        result.truncate(len);
        for value in &result {
            validate_scalar(coolprop, "Props1SImulti", *value)?;
        }
        Ok(result)
    })
}

fn join_coolprop_list(values: &[impl AsRef<str>], field: &'static str) -> Result<CString> {
    if values.is_empty() {
        return Err(Error::EmptyInput { what: field });
    }

    let mut joined = String::new();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            joined.push(',');
        }
        joined.push_str(value.as_ref());
    }
    c_string(&joined, field)
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
    fn props_si_multi_batches_states() {
        let values = props_si_multi_pure(
            &["Dmass", "Hmass"],
            "T",
            &[300.0, 310.0],
            "P",
            &[101_325.0, 101_325.0],
            "Water",
        )
        .unwrap();

        assert_eq!(values.rows().len(), 2);
        assert_eq!(values.rows()[0].len(), 2);
        assert!(values.get(0, 0).unwrap() > values.get(1, 0).unwrap());
        assert!(values.get(1, 1).unwrap() > values.get(0, 1).unwrap());
    }

    #[test]
    fn props1_si_multi_reads_trivial_outputs() {
        let values = props1_si_multi(&["Tcrit", "pcrit"], "", &["Water"], &[1.0]).unwrap();
        assert_eq!(values.len(), 2);
        assert!(values[0] > 600.0);
        assert!(values[1] > 20.0e6);
    }
}
