use coolprop_rs::{props_si_multi_pure, AbstractState, InputPair, Parameter};

fn main() -> coolprop_rs::Result<()> {
    let temperatures = [300.0, 310.0, 320.0];
    let pressures = [101_325.0; 3];

    let high_level = props_si_multi_pure(
        &["Dmass", "Hmass"],
        "T",
        &temperatures,
        "P",
        &pressures,
        "Water",
    )?;

    for (index, row) in high_level.rows().iter().enumerate() {
        println!(
            "state {index}: density = {:.6} kg/m^3, enthalpy = {:.6} J/kg",
            row[0], row[1]
        );
    }

    let mut water = AbstractState::new("HEOS", "Water")?;
    let density = water.update_and_output(
        InputPair::PressureTemperature,
        &pressures,
        &temperatures,
        Parameter::MassDensity,
    )?;
    println!("low-level densities = {density:?}");

    Ok(())
}
