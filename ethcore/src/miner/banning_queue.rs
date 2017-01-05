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

//! Banning Queue
//! Transacton Queue wrapper maintaining additional list of banned senders and contract hashes.

use std::time::Duration;
use std::ops::{Deref, DerefMut};
use std::cell::Cell;
use transaction::{SignedTransaction, Action};
use transient_hashmap::TransientHashMap;
use miner::{TransactionQueue, TransactionImportResult, TransactionOrigin, AccountDetails};
use miner::transaction_queue::InsertionTime;
use error::{Error, TransactionError};
use util::{Uint, U256, H256, Address, Hashable};

type Count = u16;

/// Auto-Banning threshold
pub enum Threshold {
	/// Should ban after given number of misbehaves reported.
	BanAfter(Count),
	/// Should never ban anything
	NeverBan
}

impl Default for Threshold {
	fn default() -> Self {
		Threshold::NeverBan
	}
}

/// Transaction queue with banlist.
pub struct BanningTransactionQueue {
	queue: TransactionQueue,
	ban_threshold: Threshold,
	senders_bans: TransientHashMap<Address, Cell<Count>>,
	recipients_bans: TransientHashMap<Address, Cell<Count>>,
	codes_bans: TransientHashMap<H256, Cell<Count>>,
}

impl BanningTransactionQueue {
	/// Creates new banlisting transaction queue
	pub fn new(queue: TransactionQueue, ban_threshold: Threshold, ban_lifetime: Duration) -> Self {
		let ban_lifetime_sec = ban_lifetime.as_secs();
		assert!(ban_lifetime_sec > 0, "Lifetime has to be specified in seconds.");
		BanningTransactionQueue {
			queue: queue,
			ban_threshold: ban_threshold,
			senders_bans: TransientHashMap::new(ban_lifetime_sec),
			recipients_bans: TransientHashMap::new(ban_lifetime_sec),
			codes_bans: TransientHashMap::new(ban_lifetime_sec),
		}
	}

	/// Borrows internal queue.
	/// NOTE: you can insert transactions to the queue even
	/// if they would be rejected because of ban otherwise.
	/// But probably you shouldn't.
	pub fn queue(&mut self) -> &mut TransactionQueue {
		&mut self.queue
	}

	/// Add to the queue taking bans into consideration.
	/// May reject transaction because of the banlist.
	pub fn add_with_banlist<F, G>(
		&mut self,
		transaction: SignedTransaction,
		time: InsertionTime,
		account_details: &F,
		gas_estimator: &G,
	) -> Result<TransactionImportResult, Error> where
		F: Fn(&Address) -> AccountDetails,
		G: Fn(&SignedTransaction) -> U256,
	{
		if let Threshold::BanAfter(threshold) = self.ban_threshold {
			// NOTE In all checks use direct query to avoid increasing ban timeout.

			// Check sender
			if let Ok(sender) = transaction.sender() {
				let count = self.senders_bans.direct().get(&sender).map(|v| v.get()).unwrap_or(0);
				if count > threshold {
					debug!(target: "txqueue", "Ignoring transaction {:?} because sender is banned.", transaction.hash());
					return Err(Error::Transaction(TransactionError::SenderBanned));
				}
			}

			// Check recipient
			if let Action::Call(recipient) = transaction.action {
				let count = self.recipients_bans.direct().get(&recipient).map(|v| v.get()).unwrap_or(0);
				if count > threshold {
					debug!(target: "txqueue", "Ignoring transaction {:?} because recipient is banned.", transaction.hash());
					return Err(Error::Transaction(TransactionError::RecipientBanned));
				}
			}

			// Check code
			if let Action::Create = transaction.action {
				let code_hash = transaction.data.sha3();
				let count = self.codes_bans.direct().get(&code_hash).map(|v| v.get()).unwrap_or(0);
				if count > threshold {
					debug!(target: "txqueue", "Ignoring transaction {:?} because code is banned.", transaction.hash());
					return Err(Error::Transaction(TransactionError::CodeBanned));
				}
			}
		}
		self.queue.add(transaction, TransactionOrigin::External, time, None, account_details, gas_estimator)
	}

	/// Ban transaction with given hash.
	/// Transaction has to be in the queue.
	///
	/// Bans sender and recipient/code and returns `true` when any ban has reached threshold.
	pub fn ban_transaction(&mut self, hash: &H256) -> bool {
		let transaction = self.queue.find(hash);
		match transaction {
			Some(transaction) => {
				let sender = transaction.sender().expect("Transaction is in queue, so the sender is already validated; qed");
				// Ban sender
				let sender_banned = self.ban_sender(sender);
				// Ban recipient and codehash
				let recipient_or_code_banned = match transaction.action {
					Action::Call(recipient) => {
						self.ban_recipient(recipient)
					},
					Action::Create => {
						self.ban_codehash(transaction.data.sha3())
					},
				};
				sender_banned || recipient_or_code_banned
			},
			None => false,
		}
	}

	/// Ban given sender.
	/// If bans threshold is reached all subsequent transactions from this sender will be rejected.
	/// Reaching bans threshold also removes all existsing transaction from this sender that are already in the
	/// queue.
	fn ban_sender(&mut self, address: Address) -> bool {
		let count = {
			let mut count = self.senders_bans.entry(address).or_insert_with(|| Cell::new(0));
			*count.get_mut() = count.get().saturating_add(1);
			count.get()
		};
		match self.ban_threshold {
			Threshold::BanAfter(threshold) if count > threshold => {
				// Banlist the sender.
				// Remove all transactions from the queue.
				self.remove_all(address, !U256::zero());
				true
			},
			_ => false
		}
	}

	/// Ban given recipient.
	/// If bans threshold is reached all subsequent transactions to this address will be rejected.
	/// Returns true if bans threshold has been reached.
	fn ban_recipient(&mut self, address: Address) -> bool {
		let count = {
			let mut count = self.recipients_bans.entry(address).or_insert_with(|| Cell::new(0));
			*count.get_mut() = count.get().saturating_add(1);
			count.get()
		};
		match self.ban_threshold {
			// TODO [ToDr] Consider removing other transactions to the same recipient from the queue?
			Threshold::BanAfter(threshold) if count > threshold => true,
			_ => false
		}
	}


	/// Ban given codehash.
	/// If bans threshold is reached all subsequent transactions to contracts with this codehash will be rejected.
	/// Returns true if bans threshold has been reached.
	fn ban_codehash(&mut self, code_hash: H256) -> bool {
		let mut count = self.codes_bans.entry(code_hash).or_insert_with(|| Cell::new(0));
		*count.get_mut() = count.get().saturating_add(1);

		match self.ban_threshold {
			// TODO [ToDr] Consider removing other transactions with the same code from the queue?
			Threshold::BanAfter(threshold) if count.get() > threshold => true,
			_ => false,
		}
	}
}

impl Deref for BanningTransactionQueue {
	type Target = TransactionQueue;

	fn deref(&self) -> &Self::Target {
		&self.queue
	}
}
impl DerefMut for BanningTransactionQueue {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.queue()
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;
	use super::{BanningTransactionQueue, Threshold};
	use ethkey::{Random, Generator};
	use transaction::{Transaction, SignedTransaction, Action};
	use error::{Error, TransactionError};
	use client::TransactionImportResult;
	use miner::{TransactionQueue, TransactionOrigin, AccountDetails};
	use util::{Uint, U256, Address, FromHex, Hashable};

	fn queue() -> BanningTransactionQueue {
		BanningTransactionQueue::new(TransactionQueue::default(), Threshold::BanAfter(1), Duration::from_secs(180))
	}

	fn default_account_details(_address: &Address) -> AccountDetails {
		AccountDetails {
			nonce: U256::zero(),
			balance: !U256::zero(),
		}
	}

	fn gas_required(_tx: &SignedTransaction) -> U256 {
		0.into()
	}

	fn transaction(action: Action) -> SignedTransaction {
		let keypair = Random.generate().unwrap();
		Transaction {
			action: action,
			value: U256::from(100),
			data: "3331600055".from_hex().unwrap(),
			gas: U256::from(100_000),
			gas_price: U256::from(10),
			nonce: U256::from(0),
		}.sign(keypair.secret(), None)
	}

	fn unwrap_err(res: Result<TransactionImportResult, Error>) -> TransactionError {
		match res {
			Err(Error::Transaction(e)) => e,
			Ok(x) => panic!("Expected error, got: Ok({:?})", x),
			Err(e) => panic!("Unexpected error type returned by queue: {:?}", e),
		}
	}

	#[test]
	fn should_allow_to_borrow_the_queue() {
		// given
		let tx = transaction(Action::Create);
		let mut txq = queue();

		// when
		txq.queue().add(tx, TransactionOrigin::External, 0, None, &default_account_details, &gas_required).unwrap();

		// then
		// should also deref to queue
		assert_eq!(txq.status().pending, 1);
	}

	#[test]
	fn should_not_accept_transactions_from_banned_sender() {
		// given
		let tx = transaction(Action::Create);
		let mut txq = queue();
		// Banlist once (threshold not reached)
		let banlist1 = txq.ban_sender(tx.sender().unwrap());
		assert!(!banlist1, "Threshold not reached yet.");
		// Insert once
		let import1 = txq.add_with_banlist(tx.clone(), 0, &default_account_details, &gas_required).unwrap();
		assert_eq!(import1, TransactionImportResult::Current);

		// when
		let banlist2 = txq.ban_sender(tx.sender().unwrap());
		let import2 = txq.add_with_banlist(tx.clone(), 0, &default_account_details, &gas_required);

		// then
		assert!(banlist2, "Threshold should be reached - banned.");
		assert_eq!(unwrap_err(import2), TransactionError::SenderBanned);
		// Should also remove transacion from the queue
		assert_eq!(txq.find(&tx.hash()), None);
	}

	#[test]
	fn should_not_accept_transactions_to_banned_recipient() {
		// given
		let recipient = Address::default();
		let tx = transaction(Action::Call(recipient));
		let mut txq = queue();
		// Banlist once (threshold not reached)
		let banlist1 = txq.ban_recipient(recipient);
		assert!(!banlist1, "Threshold not reached yet.");
		// Insert once
		let import1 = txq.add_with_banlist(tx.clone(), 0, &default_account_details, &gas_required).unwrap();
		assert_eq!(import1, TransactionImportResult::Current);

		// when
		let banlist2 = txq.ban_recipient(recipient);
		let import2 = txq.add_with_banlist(tx.clone(), 0, &default_account_details, &gas_required);

		// then
		assert!(banlist2, "Threshold should be reached - banned.");
		assert_eq!(unwrap_err(import2), TransactionError::RecipientBanned);
	}

	#[test]
	fn should_not_accept_transactions_with_banned_code() {
		// given
		let tx = transaction(Action::Create);
		let codehash = tx.data.sha3();
		let mut txq = queue();
		// Banlist once (threshold not reached)
		let banlist1 = txq.ban_codehash(codehash);
		assert!(!banlist1, "Threshold not reached yet.");
		// Insert once
		let import1 = txq.add_with_banlist(tx.clone(), 0, &default_account_details, &gas_required).unwrap();
		assert_eq!(import1, TransactionImportResult::Current);

		// when
		let banlist2 = txq.ban_codehash(codehash);
		let import2 = txq.add_with_banlist(tx.clone(), 0, &default_account_details, &gas_required);

		// then
		assert!(banlist2, "Threshold should be reached - banned.");
		assert_eq!(unwrap_err(import2), TransactionError::CodeBanned);
	}
}
