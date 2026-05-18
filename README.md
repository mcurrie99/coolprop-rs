# coolprop-rs

Safe Rust interface for [CoolProp](https://coolprop.org/).

This crate wraps the public `CoolPropLib.h` shared-library ABI and exposes CoolProp-style APIs together with Rust-style typed helpers:

- High-level calls matching CoolProp examples: `PropsSI`, `PropsSImulti`, `Props1SI`, `Props1SImulti`, `PhaseSI`, `HAPropsSI`
- Rust-style aliases: `props_si`, `props_si_multi`, `phase_si`, `ha_props_si`
- A safe `AbstractState` wrapper for repeated low-level state updates
- Typed `InputPair`, `Parameter`, `StateUpdate`, `Phase`, and saturation enums
- `Result`-based errors instead of CoolProp sentinel values
- `Drop` cleanup for `AbstractState` handles
- Thread-safe default behavior through a process-wide CoolProp library lock

The dependency on `coolprop-sys` provides the native CoolProp dynamic library for supported platforms.

## Quick Start

```rust
use coolprop_rs::{AbstractState, Parameter, PropsSI, StateUpdate};

fn main() -> coolprop_rs::Result<()> {
    let t = PropsSI("T", "P", 101_325.0, "Q", 0.0, "Water")?;
    println!("Water saturation temperature = {t:.6} K");

    let mut water = AbstractState::new("HEOS", "Water")?;
    water.update_state(StateUpdate::pressure_temperature(101_325.0, 300.0))?;

    let cp = water.keyed_output(Parameter::MassSpecificHeatConstantPressure)?;
    println!("cp = {cp:.6} J/kg/K");

    Ok(())
}
```

## Batch Calls

Use `props_si_multi`/`PropsSImulti` for high-throughput high-level calls. Rows are states, columns are requested outputs.

```rust
use coolprop_rs::props_si_multi_pure;

fn main() -> coolprop_rs::Result<()> {
    let out = props_si_multi_pure(
        &["Dmass", "Hmass"],
        "T",
        &[300.0, 310.0],
        "P",
        &[101_325.0, 101_325.0],
        "Water",
    )?;

    println!("density at first state = {}", out.get(0, 0).unwrap());
    println!("enthalpy at second state = {}", out.get(1, 1).unwrap());
    Ok(())
}
```

## AbstractState Coverage

The low-level wrapper includes:

- `factory`, `free`, `update`, `keyed_output`, typed `StateUpdate`
- batch update helpers: common outputs, one output, and five outputs
- phase specification and phase queries
- first and second partial derivatives
- saturation derivatives and saturated liquid/vapor outputs
- mole fractions, saturated mole fractions, fugacity, fugacity coefficient
- binary interaction and cubic alpha setters
- phase envelope, spinodal, and critical-point data extraction
- backend/fluid metadata helpers

## Thread Safety

Default behavior is conservative and safe: all FFI calls are serialized behind a process-wide mutex. This protects CoolProp's shared C wrapper state, including the global handle manager and global error string. Multiple Rust threads can call this crate safely; calls enter CoolProp one at a time.

`AbstractState` is `Send` but intentionally not `Sync`. Move a state to another thread, or wrap it in your own synchronization if you need shared access.

For workloads where you have validated that your CoolProp build and selected backends are reentrant, enable:

```toml
coolprop-rs = { version = "0.1", features = ["unsafe-reentrant"] }
```

That feature preloads the fluid library and removes the process-wide mutex so independent threads can call CoolProp concurrently. It is opt-in because upstream's C ABI does not provide a universal reentrancy guarantee for all functions and backends.

## Metadata And Configuration

Global helpers include `version`, `git_revision`, `fluids_list`, `predefined_mixtures_list`, `parameter_information_string`, `fluid_param_string`, `extract_backend`, `saturation_ancillary`, reference-state setters, debug-level helpers, and config setters.

## References

- [CoolProp High-Level Interface](https://coolprop.org/coolprop/HighLevelAPI.html)
- [CoolProp Low-Level Interface](https://coolprop.org/coolprop/LowLevelAPI.html)
- [CoolPropLib.h Doxygen Reference](https://coolprop.org/_static/doxygen/html/_cool_prop_lib_8h.html)
