#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Parameter {
    Temperature,
    Pressure,
    Quality,
    MassDensity,
    MolarDensity,
    MassEnthalpy,
    MolarEnthalpy,
    MassEntropy,
    MolarEntropy,
    MassInternalEnergy,
    MolarInternalEnergy,
    MassSpecificHeatConstantPressure,
    MolarSpecificHeatConstantPressure,
    MassSpecificHeatConstantVolume,
    MolarSpecificHeatConstantVolume,
    SpeedOfSound,
    Viscosity,
    Conductivity,
    Prandtl,
    MolarMass,
    SurfaceTension,
    CompressibilityFactor,
    CriticalTemperature,
    CriticalPressure,
    CriticalDensity,
}

impl Parameter {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Temperature => "T",
            Self::Pressure => "P",
            Self::Quality => "Q",
            Self::MassDensity => "Dmass",
            Self::MolarDensity => "Dmolar",
            Self::MassEnthalpy => "Hmass",
            Self::MolarEnthalpy => "Hmolar",
            Self::MassEntropy => "Smass",
            Self::MolarEntropy => "Smolar",
            Self::MassInternalEnergy => "Umass",
            Self::MolarInternalEnergy => "Umolar",
            Self::MassSpecificHeatConstantPressure => "Cpmass",
            Self::MolarSpecificHeatConstantPressure => "Cpmolar",
            Self::MassSpecificHeatConstantVolume => "Cvmass",
            Self::MolarSpecificHeatConstantVolume => "Cvmolar",
            Self::SpeedOfSound => "A",
            Self::Viscosity => "V",
            Self::Conductivity => "L",
            Self::Prandtl => "Prandtl",
            Self::MolarMass => "M",
            Self::SurfaceTension => "I",
            Self::CompressibilityFactor => "Z",
            Self::CriticalTemperature => "Tcrit",
            Self::CriticalPressure => "Pcrit",
            Self::CriticalDensity => "rhocrit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputPair {
    /// value1 = pressure [Pa], value2 = temperature [K]
    PressureTemperature,
    /// value1 = pressure [Pa], value2 = vapor quality [mol/mol]
    PressureQuality,
    /// value1 = vapor quality [mol/mol], value2 = temperature [K]
    QualityTemperature,
    /// value1 = mass density [kg/m^3], value2 = temperature [K]
    MassDensityTemperature,
    /// value1 = molar density [mol/m^3], value2 = temperature [K]
    MolarDensityTemperature,
    /// value1 = mass enthalpy [J/kg], value2 = pressure [Pa]
    MassEnthalpyPressure,
    /// value1 = molar enthalpy [J/mol], value2 = pressure [Pa]
    MolarEnthalpyPressure,
    /// value1 = pressure [Pa], value2 = mass entropy [J/kg/K]
    PressureMassEntropy,
    /// value1 = pressure [Pa], value2 = molar entropy [J/mol/K]
    PressureMolarEntropy,
    /// value1 = mass internal energy [J/kg], value2 = mass density [kg/m^3]
    MassInternalEnergyMassDensity,
    /// value1 = molar internal energy [J/mol], value2 = molar density [mol/m^3]
    MolarInternalEnergyMolarDensity,
}

impl InputPair {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PressureTemperature => "PT_INPUTS",
            Self::PressureQuality => "PQ_INPUTS",
            Self::QualityTemperature => "QT_INPUTS",
            Self::MassDensityTemperature => "DmassT_INPUTS",
            Self::MolarDensityTemperature => "DmolarT_INPUTS",
            Self::MassEnthalpyPressure => "HmassP_INPUTS",
            Self::MolarEnthalpyPressure => "HmolarP_INPUTS",
            Self::PressureMassEntropy => "PSmass_INPUTS",
            Self::PressureMolarEntropy => "PSmolar_INPUTS",
            Self::MassInternalEnergyMassDensity => "DmassUmass_INPUTS",
            Self::MolarInternalEnergyMolarDensity => "DmolarUmolar_INPUTS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PhaseSpecifier {
    Liquid,
    Gas,
    TwoPhase,
    Supercritical,
    SupercriticalGas,
    SupercriticalLiquid,
}

impl PhaseSpecifier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Liquid => "phase_liquid",
            Self::Gas => "phase_gas",
            Self::TwoPhase => "phase_twophase",
            Self::Supercritical => "phase_supercritical",
            Self::SupercriticalGas => "phase_supercritical_gas",
            Self::SupercriticalLiquid => "phase_supercritical_liquid",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Phase {
    Liquid,
    Gas,
    TwoPhase,
    Supercritical,
    SupercriticalGas,
    SupercriticalLiquid,
    CriticalPoint,
    Unknown,
    Other(String),
}

impl Phase {
    pub fn from_coolprop(value: impl Into<String>) -> Self {
        let value = value.into();
        let normalized = value
            .strip_prefix("phase_")
            .or_else(|| value.strip_prefix("iphase_"))
            .unwrap_or(&value);

        match normalized {
            "liquid" => Self::Liquid,
            "gas" => Self::Gas,
            "twophase" => Self::TwoPhase,
            "supercritical" => Self::Supercritical,
            "supercritical_gas" => Self::SupercriticalGas,
            "supercritical_liquid" => Self::SupercriticalLiquid,
            "critical_point" => Self::CriticalPoint,
            "unknown" => Self::Unknown,
            _ => Self::Other(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReferenceState {
    Default,
    Ashrae,
    Iir,
    Nbp,
}

impl ReferenceState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "DEF",
            Self::Ashrae => "ASHRAE",
            Self::Iir => "IIR",
            Self::Nbp => "NBP",
        }
    }
}
