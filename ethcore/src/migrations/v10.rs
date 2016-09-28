// Copyright 2015, 2016 Ethcore (UK) Ltd.
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

//! This migration compresses the state db.

use util::migration::SimpleMigration;

/// Compressing migration.
#[derive(Default)]
pub struct ToV10;

impl SimpleMigration for ToV10 {
	fn version(&self) -> u32 {
		10
	}

	fn columns(&self) -> Option<u32> { Some(6) }

	fn simple_migrate(&mut self, _key: Vec<u8>, _value: Vec<u8>) -> Option<(Vec<u8>, Vec<u8>)> {
		None
	}
}
