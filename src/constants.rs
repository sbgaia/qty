//! Physical constants, for [`formula!`](crate::formula!).
//!
//! Each is an [`Expr`] in coherent SI units, so it can have dimensions that
//! no quantity has, like the gravitational constant. The values are the SI
//! defining constants and the CODATA 2022 recommended values.
//!
//! ```rust
//! use qty::*;
//! use qty::constants::{PLANCK_CONSTANT as H, SPEED_OF_LIGHT as C};
//!
//! // The energy of a green photon.
//! let wavelength = Length::from_nanometers(532.0);
//! let energy: Energy = formula!(H * C / wavelength);
//! assert_eq!(format!("{:.3}", energy.in_unit(EnergyUnit::ElectronVolt)), "2.331 eV");
//! ```
//!
//! A constant that is a quantity converts to it with `into()`:
//!
//! ```rust
//! # use qty::*;
//! let c: Speed = qty::constants::SPEED_OF_LIGHT.into();
//! assert_eq!(c.as_kilometers_per_second(), 299_792.458);
//! ```

use crate::formula::{Dims, Expr};

/// The speed of light in vacuum, c, in m/s. Exact.
pub const SPEED_OF_LIGHT: Expr<Dims<1, 0, -1>> = Expr::new(299_792_458.0);

/// The Planck constant, h, in J·s. Exact.
pub const PLANCK_CONSTANT: Expr<Dims<2, 1, -1>> = Expr::new(6.626_070_15e-34);

/// The reduced Planck constant, ħ = h / 2π, in J·s.
pub const REDUCED_PLANCK_CONSTANT: Expr<Dims<2, 1, -1>> = Expr::new(1.054_571_817_646_156_4e-34);

/// The elementary charge, e, in C. Exact.
pub const ELEMENTARY_CHARGE: Expr<Dims<0, 0, 1, 1>> = Expr::new(1.602_176_634e-19);

/// The Boltzmann constant, k, in J/K. Exact.
pub const BOLTZMANN_CONSTANT: Expr<Dims<2, 1, -2, 0, -1>> = Expr::new(1.380_649e-23);

/// The Avogadro constant, Nₐ, in mol⁻¹. Exact.
pub const AVOGADRO_CONSTANT: Expr<Dims<0, 0, 0, 0, 0, -1>> = Expr::new(6.022_140_76e23);

/// The molar gas constant, R = Nₐk, in J/(mol·K).
pub const MOLAR_GAS_CONSTANT: Expr<Dims<2, 1, -2, 0, -1, -1>> = Expr::new(8.314_462_618_153_24);

/// The Faraday constant, F = Nₐe, in C/mol.
pub const FARADAY_CONSTANT: Expr<Dims<0, 0, 1, 1, 0, -1>> = Expr::new(96_485.332_123_310_02);

/// The Stefan–Boltzmann constant, σ, in W/(m²·K⁴).
pub const STEFAN_BOLTZMANN_CONSTANT: Expr<Dims<0, 1, -3, 0, -4>> =
    Expr::new(5.670_374_419_184_429e-8);

/// The Newtonian constant of gravitation, G, in m³/(kg·s²).
pub const GRAVITATIONAL_CONSTANT: Expr<Dims<3, -1, -2>> = Expr::new(6.674_30e-11);

/// The vacuum electric permittivity, ε₀, in F/m.
pub const VACUUM_PERMITTIVITY: Expr<Dims<-3, -1, 4, 2>> = Expr::new(8.854_187_818_8e-12);

/// The vacuum magnetic permeability, µ₀, in N/A².
pub const VACUUM_PERMEABILITY: Expr<Dims<1, 1, -2, -2>> = Expr::new(1.256_637_061_27e-6);

/// The Coulomb constant, kₑ = 1 / 4πε₀, in N·m²/C².
pub const COULOMB_CONSTANT: Expr<Dims<3, 1, -4, -2>> = Expr::new(8.987_551_786_2e9);

/// The mass of the electron, in kg.
pub const ELECTRON_MASS: Expr<Dims<0, 1>> = Expr::new(9.109_383_713_9e-31);

/// The mass of the proton, in kg.
pub const PROTON_MASS: Expr<Dims<0, 1>> = Expr::new(1.672_621_925_95e-27);

/// The atomic mass constant, one twelfth of the mass of carbon-12, in kg.
pub const ATOMIC_MASS_CONSTANT: Expr<Dims<0, 1>> = Expr::new(1.660_539_068_92e-27);

/// Standard gravity, g₀, in m/s². Exact by definition.
pub const STANDARD_GRAVITY: Expr<Dims<1, 0, -2>> = Expr::new(9.806_65);

/// The standard atmosphere, in Pa. Exact by definition.
pub const STANDARD_ATMOSPHERE: Expr<Dims<-1, 1, -2>> = Expr::new(101_325.0);
