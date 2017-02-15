// Copyright 2015-2017 Parity Technologies (UK) Ltd.
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

//! Manages local node data: pending local transactions, sync security level

use std::sync::Arc;
use std::fmt;

use ethcore::transaction::{
	SignedTransaction, PendingTransaction, UnverifiedTransaction,
	Condition as TransactionCondition
};
use rlp::{UntrustedRlp, View};
use util::kvdb::KeyValueDB;

extern crate ethcore;
extern crate ethcore_util as util;
extern crate rlp;
extern crate serde_json;
extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

#[cfg(test)]
extern crate ethkey;

const LOCAL_TRANSACTIONS_KEY: &'static [u8] = &*b"LOCAL_TXS";

/// Errors which can occur while using the local data store.
#[derive(Debug)]
pub enum Error {
	/// Database errors: these manifest as `String`s.
	Database(String),
	/// JSON errors.
	Json(::serde_json::Error),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Error::Database(ref val) => write!(f, "{}", val),
			Error::Json(ref err) => write!(f, "{}", err),
		}
	}
}

#[derive(Serialize, Deserialize)]
enum Condition {
	Number(::ethcore::header::BlockNumber),
	Timestamp(u64),
}

impl From<TransactionCondition> for Condition {
	fn from(cond: TransactionCondition) -> Self {
		match cond {
			TransactionCondition::Number(num) => Condition::Number(num),
			TransactionCondition::Timestamp(tm) => Condition::Timestamp(tm),
		}
	}
}

impl Into<TransactionCondition> for Condition {
	fn into(self) -> TransactionCondition {
		match self {
			Condition::Number(num) => TransactionCondition::Number(num),
			Condition::Timestamp(tm) => TransactionCondition::Timestamp(tm),
		}
	}
}

#[derive(Serialize, Deserialize)]
struct TransactionEntry {
	rlp_bytes: Vec<u8>,
	condition: Option<Condition>,
}

impl TransactionEntry {
	fn into_pending(self) -> Option<PendingTransaction> {
		let tx: UnverifiedTransaction = match UntrustedRlp::new(&self.rlp_bytes).as_val() {
			Err(e) => {
				warn!(target: "local_store", "Invalid persistent transaction stored: {}", e);
				return None
			}
			Ok(tx) => tx,
		};

		let hash = tx.hash();
		match SignedTransaction::new(tx) {
			Ok(tx) => Some(PendingTransaction::new(tx, self.condition.map(Into::into))),
			Err(_) => {
				warn!(target: "local_store", "Bad signature on persistent transaction: {}", hash);
				return None
			}
		}
	}
}

impl From<PendingTransaction> for TransactionEntry {
	fn from(pending: PendingTransaction) -> Self {
		TransactionEntry {
			rlp_bytes: ::rlp::encode(&pending.transaction).to_vec(),
			condition: pending.condition.map(Into::into),
		}
	}
}

/// Something which can provide information about the local node.
pub trait NodeInfo {
	/// Get all pending transactions of local origin.
	fn pending_transactions(&self) -> Vec<PendingTransaction>;
}

/// Manages local node data.
///
/// In specific, this will be used to store things like unpropagated local transactions
/// and the node security level.
pub struct LocalDataStore<T: NodeInfo> {
	db: Arc<KeyValueDB>,
	col: Option<u32>,
	node: T,
}

impl<T: NodeInfo> LocalDataStore<T> {
	/// Create a new local data store, given a database, a column to write to, and a node.
	/// Attempts to read data out of the store, and move it into the node.
	pub fn create(db: Arc<KeyValueDB>, col: Option<u32>, node: T) -> Self {
		LocalDataStore {
			db: db,
			col: col,
			node: node,
		}
	}

	/// Attempt to read pending transactions out of the local store.
	pub fn pending_transactions(&self) -> Result<Vec<PendingTransaction>, Error> {
		if let Some(val) = self.db.get(self.col, LOCAL_TRANSACTIONS_KEY).map_err(Error::Database)? {
			let local_txs: Vec<_> = ::serde_json::from_slice::<Vec<TransactionEntry>>(&val)
				.map_err(Error::Json)?
				.into_iter()
				.filter_map(TransactionEntry::into_pending)
				.collect();

			Ok(local_txs)
		} else {
			Ok(Vec::new())
		}
	}

	/// Update the entries in the database.
	pub fn update(&self) -> Result<(), Error> {
		let mut batch = self.db.transaction();

		let local_entries: Vec<TransactionEntry> = self.node.local_pending_transactions()
			.into_iter()
			.map(Into::into)
			.collect();

		let local_json = ::serde_json::to_value(&local_entries).map_err(Error::Json)?;
		let json_str = format!("{}", local_json);

		batch.put_vec(self.col, LOCAL_TRANSACTIONS_KEY, json_str.into_bytes());
		self.db.write(batch).map_err(Error::Database)
	}
}

impl<T: NodeInfo> Drop for LocalDataStore<T> {
	fn drop(&mut self) {
		debug!(target: "local_store", "Updating node data store on shutdown.");

		let _ = self.update();
	}
}

#[cfg(test)]
mod tests {
	use super::{NodeInfo, LocalDataStore};

	use std::sync::Arc;
	use ethcore::transaction::{Transaction, Condition, PendingTransaction};
	use ethkey::{Brain, Generator};

	// we want to test: round-trip of good transactions.
	// failure to roundtrip bad transactions (but that it doesn't panic)

	struct Dummy(Vec<PendingTransaction>);
	impl NodeInfo for Dummy {
		fn local_pending_transactions(&self) -> Vec<PendingTransaction> { self.0.clone() }
	}

	#[test]
	fn twice_empty() {
		let db = Arc::new(::util::kvdb::in_memory(0));

		{
			let store = LocalDataStore::create(db.clone(), None, Dummy(vec![]));
			assert_eq!(store.pending_transactions().unwrap(), vec![])
		}

		{
			let store = LocalDataStore::create(db.clone(), None, Dummy(vec![]));
			assert_eq!(store.pending_transactions().unwrap(), vec![])
		}
	}

	#[test]
	fn with_condition() {
		let keypair = Brain::new("abcd".into()).generate().unwrap();
		let transactions: Vec<_> = (0..10u64).map(|nonce| {
			let mut tx = Transaction::default();
			tx.nonce = nonce.into();

			let signed = tx.sign(keypair.secret(), None);
			let condition = match nonce {
				5 => Some(Condition::Number(100_000)),
				_ => None,
			};

			PendingTransaction::new(signed, condition)
		}).collect();

		let db = Arc::new(::util::kvdb::in_memory(0));

		{
			// nothing written yet, will write pending.
			let store = LocalDataStore::create(db.clone(), None, Dummy(transactions.clone()));
			assert_eq!(store.pending_transactions().unwrap(), vec![])
		}
		{
			// pending written, will write nothing.
			let store = LocalDataStore::create(db.clone(), None, Dummy(vec![]));
			assert_eq!(store.pending_transactions().unwrap(), transactions)
		}
		{
			// pending removed, will write nothing.
			let store = LocalDataStore::create(db.clone(), None, Dummy(vec![]));
			assert_eq!(store.pending_transactions().unwrap(), vec![])
		}
	}

	#[test]
	#[should_panic]
	fn bad_transactions() {
		let transactions: Vec<_> = (0..10u64).map(|nonce| {
			let mut tx = Transaction::default();
			tx.nonce = nonce.into();

			let signed = tx.fake_sign(Default::default());
			PendingTransaction::new(signed, None)
		}).collect();

		let db = Arc::new(::util::kvdb::in_memory(0));
		{
			// nothing written, will write bad.
			let store = LocalDataStore::create(db.clone(), None, Dummy(transactions.clone()));
			assert_eq!(store.pending_transactions().unwrap(), vec![])
		}
		{
			// try to load bad. they will be skipped, leading to empty vector.
			let store = LocalDataStore::create(db.clone(), None, Dummy(vec![]));
			assert_eq!(store.pending_transactions().unwrap(), transactions)
		}
	}
}
