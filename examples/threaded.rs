use std::thread;

use coolprop_rs::{props_si, AbstractState, InputPair};

fn main() -> coolprop_rs::Result<()> {
    let mut workers = Vec::new();

    for offset in 0..4 {
        workers.push(thread::spawn(move || -> coolprop_rs::Result<f64> {
            let temperature = 300.0 + offset as f64;
            props_si("Dmass", "T", temperature, "P", 101_325.0, "Water")
        }));
    }

    for worker in workers {
        println!("density = {:.6} kg/m^3", worker.join().unwrap()?);
    }

    let mut state = AbstractState::new("HEOS", "Water")?;
    state.update(InputPair::PressureTemperature, 101_325.0, 300.0)?;
    println!("cp = {:.6} J/kg/K", state.keyed_output_by_name("Cpmass")?);

    Ok(())
}
