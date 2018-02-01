// Copyright 2017 Parity Technologies (UK) Ltd.
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
// along with Parity. If not, see <http://www.gnu.org/licenses/>.

//! A service transactions contract checker.

use parking_lot::Mutex;
use ethereum_types::Address;
use transaction::SignedTransaction;
use types::ids::BlockId;

use_contract!(service_transaction, "ServiceTransaction", "../ethcore/native_contracts/res/service_transaction.json");

const SERVICE_TRANSACTION_CONTRACT_REGISTRY_NAME: &'static str = "service_transaction_checker";

/// A contract calling interface.
pub trait ContractCaller {
	/// Returns address of contract from the registry, given it's name
	fn registry_address(&self, name: &str) -> Option<Address>;

	/// Executes a contract call at given block.
	fn call_contract(&self, BlockId, Address, Vec<u8>) -> Result<Vec<u8>, String>;
}

/// Service transactions checker.
#[derive(Default)]
pub struct ServiceTransactionChecker {
	contract: service_transaction::ServiceTransaction,
	contract_address: Mutex<Option<Address>>,
}

impl ServiceTransactionChecker {
	/// Try to create instance, reading contract address from given chain client.
	pub fn update_from_chain_client(&self, client: &ContractCaller) {
		let mut contract_address = self.contract_address.lock();
		if contract_address.is_none() {
			if let Some(address) = client.registry_address(SERVICE_TRANSACTION_CONTRACT_REGISTRY_NAME) {
				trace!(target: "txqueue", "Configuring for service transaction checker contract from {}", address);
				*contract_address = Some(address);
			}
		}
	}

	/// Checks if service transaction can be appended to the transaction queue.
	pub fn check(&self, client: &ContractCaller, tx: &SignedTransaction) -> Result<bool, String> {
		debug_assert!(tx.gas_price.is_zero());

		match *self.contract_address.lock() {
			Some(address) => {
				self.contract.functions()
					.certified()
					.call(tx.sender(), &|data| client.call_contract(BlockId::Latest, address, data))
					.map_err(|e| e.to_string())
			},
			None => Err("contract is not configured".into()),
		}
	}
}
