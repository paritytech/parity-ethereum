// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

//! Auto-updates minimal gas price requirement.

use ethereum_types::U256;
#[cfg(feature = "price-info")]
use gas_price_calibrator::GasPriceCalibrator;

/// Struct to look after updating the acceptable gas price of a miner.
#[derive(Debug, PartialEq)]
pub enum GasPricer {
	/// A fixed gas price in terms of Wei - always the argument given.
	Fixed(U256),
	#[cfg(feature = "price-info")]
	/// Gas price is calibrated according to a fixed amount of USD.
	Calibrated(GasPriceCalibrator),
}

impl GasPricer {
	#[cfg(feature = "price-info")]
	/// Create a new Calibrated `GasPricer`.
	pub fn new_calibrated(calibrator: GasPriceCalibrator) -> GasPricer {
		GasPricer::Calibrated(calibrator)
	}

	/// Create a new Fixed `GasPricer`.
	pub fn new_fixed(gas_price: U256) -> GasPricer {
		GasPricer::Fixed(gas_price)
	}

	/// Recalibrate current gas price.
	pub fn recalibrate<F: FnOnce(U256) + Sync + Send + 'static>(&mut self, set_price: F) {
		match *self {
			GasPricer::Fixed(ref max) => set_price(max.clone()),
			#[cfg(feature = "price-info")]
			GasPricer::Calibrated(ref mut cal) => cal.recalibrate(set_price),
		}
	}
}
