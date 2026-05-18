# coolprop-rs

Safe Rust interface for [CoolProp](https://coolprop.org/).

This crate wraps the public `CoolPropLib.h` shared-library ABI and exposes:

- High-level calls matching CoolProp examples: `PropsSI`, `PhaseSI`, `HAPropsSI`
- Rust-style aliases: `props_si`, `phase_si`, `ha_props_si`
- A safe `AbstractState` wrapper for repeated low-level state updates
- `Result`-based errors instead of CoolProp sentinel values
- `Drop` cleanup for `AbstractState` handles
- Thread-safe calls through a process-wide CoolProp library lock

The dependency on `coolprop-sys` provides the native CoolProp dynamic library for supported platforms.

## Example

```rust
use coolprop_rs::{PropsSI, AbstractState, InputPair, Parameter};

fn main() -> coolprop_rs::Result<()> {
    let t = PropsSI("T", "P", 101_325.0, "Q", 0.0, "Water")?;
    println!("Water saturation temperature = {t:.6} K");

    let mut water = AbstractState::new("HEOS", "Water")?;
    water.update(InputPair::PressureTemperature, 101_325.0, 300.0)?;

    let cp = water.keyed_output(Parameter::MassSpecificHeatConstantPressure)?;
    println!("cp = {cp:.6} J/kg/K");

    Ok(())
}
```

## Thread Safety

`AbstractState` is `Send` but intentionally not `Sync`. You can move a state to another thread, but shared mutable access requires your own synchronization.

All FFI calls are serialized behind a process-wide lock. This is conservative but safe: CoolProp's C ABI uses global handle and error-string state, and upstream documentation does not promise fully reentrant calls for every exported function. Multiple Rust threads can call this crate safely; calls will enter CoolProp one at a time.

## CoolProp API Coverage

Implemented now:

- `PropsSI`, `Props1SI`, `PhaseSI`, `HAPropsSI`
- global metadata helpers such as `version`, `git_revision`, `fluids_list`
- `AbstractState_factory`, `update`, `keyed_output`, phase imposition, fractions, first partial derivatives, batch common outputs, and `free`

Not implemented yet:

- Full `PropsSImulti` matrix wrapper
- Phase-envelope and spinodal extraction helpers
- Config setters beyond reference-state helpers

## References

- [CoolProp High-Level Interface](https://coolprop.org/coolprop/HighLevelAPI.html)
- [CoolProp Low-Level Interface](https://coolprop.org/coolprop/LowLevelAPI.html)
- [CoolPropLib.h Doxygen Reference](https://coolprop.org/_static/doxygen/html/_cool_prop_lib_8h.html)
