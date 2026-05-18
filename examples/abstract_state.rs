use coolprop_rs::{AbstractState, Parameter, SaturatedState, StateUpdate};

fn main() -> coolprop_rs::Result<()> {
    let mut water = AbstractState::new("HEOS", "Water")?;

    water.update_state(StateUpdate::pressure_temperature(101_325.0, 300.0))?;
    println!("density = {:.6} kg/m^3", water.mass_density()?);
    println!("viscosity = {:.12} Pa*s", water.viscosity()?);

    water.update_state(StateUpdate::pressure_quality(101_325.0, 0.0))?;
    let rho_l =
        water.keyed_output_saturated_state(SaturatedState::Liquid, Parameter::MassDensity)?;
    let rho_v =
        water.keyed_output_saturated_state(SaturatedState::Vapor, Parameter::MassDensity)?;
    println!("sat liquid density = {rho_l:.6} kg/m^3");
    println!("sat vapor density = {rho_v:.6} kg/m^3");

    let critical_points = water.all_critical_points()?;
    println!("critical points = {critical_points:?}");

    Ok(())
}
