// Copyright 2015, 2016 Parity Technologies (UK) Ltd.
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

//! A blockchain engine that supports a basic, non-BFT proof-of-authority.

use ethkey::{recover, public_to_address};
use account_provider::AccountProvider;
use block::*;
use builtin::Builtin;
use spec::CommonParams;
use engines::{Engine, Seal};
use env_info::EnvInfo;
use error::{BlockError, Error};
use evm::Schedule;
use ethjson;
use header::Header;
use transaction::SignedTransaction;

use util::*;

/// `BasicAuthority` params.
#[derive(Debug, PartialEq)]
pub struct BasicAuthorityParams {
	/// Gas limit divisor.
	pub gas_limit_bound_divisor: U256,
	/// Block duration.
	pub duration_limit: u64,
	/// Valid signatories.
	pub authorities: HashSet<Address>,
}

impl From<ethjson::spec::BasicAuthorityParams> for BasicAuthorityParams {
	fn from(p: ethjson::spec::BasicAuthorityParams) -> Self {
		BasicAuthorityParams {
			gas_limit_bound_divisor: p.gas_limit_bound_divisor.into(),
			duration_limit: p.duration_limit.into(),
			authorities: p.authorities.into_iter().map(Into::into).collect::<HashSet<_>>(),
		}
	}
}

/// Engine using `BasicAuthority` proof-of-work consensus algorithm, suitable for Ethereum
/// mainnet chains in the Olympic, Frontier and Homestead eras.
pub struct BasicAuthority {
	params: CommonParams,
	our_params: BasicAuthorityParams,
	builtins: BTreeMap<Address, Builtin>,
	account_provider: Mutex<Option<Arc<AccountProvider>>>,
	password: RwLock<Option<String>>,
}

impl BasicAuthority {
	/// Create a new instance of BasicAuthority engine
	pub fn new(params: CommonParams, our_params: BasicAuthorityParams, builtins: BTreeMap<Address, Builtin>) -> Self {
		BasicAuthority {
			params: params,
			our_params: our_params,
			builtins: builtins,
			account_provider: Mutex::new(None),
			password: RwLock::new(None),
		}
	}
}

impl Engine for BasicAuthority {
	fn name(&self) -> &str { "BasicAuthority" }
	fn version(&self) -> SemanticVersion { SemanticVersion::new(1, 0, 0) }
	// One field - the signature
	fn seal_fields(&self) -> usize { 1 }

	fn params(&self) -> &CommonParams { &self.params }
	fn builtins(&self) -> &BTreeMap<Address, Builtin> { &self.builtins }

	/// Additional engine-specific information for the user/developer concerning `header`.
	fn extra_info(&self, _header: &Header) -> BTreeMap<String, String> { map!["signature".to_owned() => "TODO".to_owned()] }

	fn schedule(&self, _env_info: &EnvInfo) -> Schedule {
		Schedule::new_homestead()
	}

	fn populate_from_parent(&self, header: &mut Header, parent: &Header, gas_floor_target: U256, _gas_ceil_target: U256) {
		header.set_difficulty(parent.difficulty().clone());
		header.set_gas_limit({
			let gas_limit = parent.gas_limit().clone();
			let bound_divisor = self.our_params.gas_limit_bound_divisor;
			if gas_limit < gas_floor_target {
				min(gas_floor_target, gas_limit + gas_limit / bound_divisor - 1.into())
			} else {
				max(gas_floor_target, gas_limit - gas_limit / bound_divisor + 1.into())
			}
		});
	}

	fn is_sealer(&self, author: &Address) -> Option<bool> {
		Some(self.our_params.authorities.contains(author))
	}

	/// Attempt to seal the block internally.
	///
	/// This operation is synchronous and may (quite reasonably) not be available, in which `false` will
	/// be returned.
	fn generate_seal(&self, block: &ExecutedBlock) -> Seal {
		if let Some(ref ap) = *self.account_provider.lock() {
			let header = block.header();
			let message = header.bare_hash();
			// account should be pernamently unlocked, otherwise sealing will fail
			if let Ok(signature) = ap.sign(*block.header().author(), self.password.read().clone(), message) {
				return Seal::Regular(vec![::rlp::encode(&(&*signature as &[u8])).to_vec()]);
			} else {
				trace!(target: "basicauthority", "generate_seal: FAIL: accounts secret key unavailable");
			}
		} else {
			trace!(target: "basicauthority", "generate_seal: FAIL: accounts not provided");
		}
		Seal::None
	}

	fn verify_block_basic(&self, header: &Header, _block: Option<&[u8]>) -> result::Result<(), Error> {
		// check the seal fields.
		// TODO: pull this out into common code.
		if header.seal().len() != self.seal_fields() {
			return Err(From::from(BlockError::InvalidSealArity(
				Mismatch { expected: self.seal_fields(), found: header.seal().len() }
			)));
		}
		Ok(())
	}

	fn verify_block_unordered(&self, header: &Header, _block: Option<&[u8]>) -> result::Result<(), Error> {
		use rlp::{UntrustedRlp, View};

		// check the signature is legit.
		let sig = UntrustedRlp::new(&header.seal()[0]).as_val::<H520>()?;
		let signer = public_to_address(&recover(&sig.into(), &header.bare_hash())?);
		if !self.our_params.authorities.contains(&signer) {
			return Err(BlockError::InvalidSeal)?;
		}
		Ok(())
	}

	fn verify_block_family(&self, header: &Header, parent: &Header, _block: Option<&[u8]>) -> result::Result<(), Error> {
		// we should not calculate difficulty for genesis blocks
		if header.number() == 0 {
			return Err(From::from(BlockError::RidiculousNumber(OutOfBounds { min: Some(1), max: None, found: header.number() })));
		}

		// Check difficulty is correct given the two timestamps.
		if header.difficulty() != parent.difficulty() {
			return Err(From::from(BlockError::InvalidDifficulty(Mismatch { expected: *parent.difficulty(), found: *header.difficulty() })))
		}
		let gas_limit_divisor = self.our_params.gas_limit_bound_divisor;
		let min_gas = parent.gas_limit().clone() - parent.gas_limit().clone() / gas_limit_divisor;
		let max_gas = parent.gas_limit().clone() + parent.gas_limit().clone() / gas_limit_divisor;
		if header.gas_limit() <= &min_gas || header.gas_limit() >= &max_gas {
			return Err(From::from(BlockError::InvalidGasLimit(OutOfBounds { min: Some(min_gas), max: Some(max_gas), found: header.gas_limit().clone() })));
		}
		Ok(())
	}

	fn verify_transaction_basic(&self, t: &SignedTransaction, _header: &Header) -> result::Result<(), Error> {
		t.check_low_s()?;
		Ok(())
	}

	fn verify_transaction(&self, t: &SignedTransaction, _header: &Header) -> Result<(), Error> {
		// TODO [ToDr]!!!!
		// t.sender().map(|_|()) // Perform EC recovery and cache sender
		Ok(())
	}

	fn set_signer(&self, _address: Address, password: String) {
		*self.password.write() = Some(password);
	}

	fn register_account_provider(&self, ap: Arc<AccountProvider>) {
		*self.account_provider.lock() = Some(ap);
	}
}

#[cfg(test)]
mod tests {
	use util::*;
	use block::*;
	use env_info::EnvInfo;
	use error::{BlockError, Error};
	use tests::helpers::*;
	use account_provider::AccountProvider;
	use header::Header;
	use spec::Spec;
	use engines::Seal;

	/// Create a new test chain spec with `BasicAuthority` consensus engine.
	fn new_test_authority() -> Spec {
		let bytes: &[u8] = include_bytes!("../../res/basic_authority.json");
		Spec::load(bytes).expect("invalid chain spec")
	}

	#[test]
	fn has_valid_metadata() {
		let engine = new_test_authority().engine;
		assert!(!engine.name().is_empty());
		assert!(engine.version().major >= 1);
	}

	#[test]
	fn can_return_schedule() {
		let engine = new_test_authority().engine;
		let schedule = engine.schedule(&EnvInfo {
			number: 10000000,
			author: 0.into(),
			timestamp: 0,
			difficulty: 0.into(),
			last_hashes: Arc::new(vec![]),
			gas_used: 0.into(),
			gas_limit: 0.into(),
		});

		assert!(schedule.stack_limit > 0);
	}

	#[test]
	fn can_do_seal_verification_fail() {
		let engine = new_test_authority().engine;
		let header: Header = Header::default();

		let verify_result = engine.verify_block_basic(&header, None);

		match verify_result {
			Err(Error::Block(BlockError::InvalidSealArity(_))) => {},
			Err(_) => { panic!("should be block seal-arity mismatch error (got {:?})", verify_result); },
			_ => { panic!("Should be error, got Ok"); },
		}
	}

	#[test]
	fn can_do_signature_verification_fail() {
		let engine = new_test_authority().engine;
		let mut header: Header = Header::default();
		header.set_seal(vec![::rlp::encode(&H520::default()).to_vec()]);

		let verify_result = engine.verify_block_unordered(&header, None);
		assert!(verify_result.is_err());
	}

	#[test]
	fn can_generate_seal() {
		let tap = AccountProvider::transient_provider();
		let addr = tap.insert_account("".sha3(), "").unwrap();

		let spec = new_test_authority();
		let engine = &*spec.engine;
		engine.set_signer(addr, "".into());
		engine.register_account_provider(Arc::new(tap));
		let genesis_header = spec.genesis_header();
		let mut db_result = get_temp_state_db();
		let db = spec.ensure_db_good(db_result.take(), &Default::default()).unwrap();
		let last_hashes = Arc::new(vec![genesis_header.hash()]);
		let b = OpenBlock::new(engine, Default::default(), false, db, &genesis_header, last_hashes, addr, (3141562.into(), 31415620.into()), vec![]).unwrap();
		let b = b.close_and_lock();
		if let Seal::Regular(seal) = engine.generate_seal(b.block()) {
			assert!(b.try_seal(engine, seal).is_ok());
		}
	}

	#[test]
	fn seals_internally() {
		let tap = AccountProvider::transient_provider();
		let authority = tap.insert_account("".sha3(), "").unwrap();

		let engine = new_test_authority().engine;
		assert!(!engine.is_sealer(&Address::default()).unwrap());
		assert!(engine.is_sealer(&authority).unwrap());
	}
}
