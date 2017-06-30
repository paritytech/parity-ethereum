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

//! JSONRPC interface for Whisper.
//!
//! Manages standard message format decoding, ephemeral identities, signing,
//! encryption, and decryption.
//!
//! Provides an interface for using whisper to transmit data securely.

use std::sync::Arc;

use jsonrpc_core::{Error, ErrorCode, Metadata};
use jsonrpc_pubsub::{Session, PubSubMetadata, SubscriptionId};
use jsonrpc_macros::pubsub;

use bigint::hash::H256;
use futures::{future, BoxFuture};
use parking_lot::{Mutex, RwLock};
use rand::{Rng, SeedableRng, XorShiftRng};

use self::filter::Filter;
use self::key_store::{Key, KeyStore};
use self::types::HexEncode;

use message::{CreateParams, Message, Topic};

mod crypto;
mod filter;
mod key_store;
mod payload;
mod types;

// create whisper RPC error.
fn whisper_error<T: Into<String>>(message: T) -> Error {
	const ERROR_CODE: i64 = -32085;

	Error {
		code: ErrorCode::ServerError(ERROR_CODE),
		message: message.into(),
		data: None,
	}
}

// abridge topic using first four bytes of hash.
fn abridge_topic(topic: &[u8]) -> Topic {
	let mut abridged = [0; 4];
	let hash = ::tiny_keccak::keccak256(topic);
	abridged.copy_from_slice(&hash[..4]);
	abridged.into()
}

build_rpc_trait! {
	/// Whisper RPC interface.
	pub trait Whisper {
		/// Generate a new asymmetric key pair and return an identity.
		#[rpc(name = "shh_newKeyPair")]
		fn new_key_pair(&self) -> Result<types::Identity, Error>;

		/// Import the given SECP2561k private key and return an identity.
		#[rpc(name = "shh_addPrivateKey")]
		fn add_private_key(&self, types::Private) -> Result<types::Identity, Error>;

		/// Generate a new symmetric key and return an identity.
		#[rpc(name = "shh_newSymKey")]
		fn new_sym_key(&self) -> Result<types::Identity, Error>;

		/// Import the given symmetric key and return an identity.
		#[rpc(name = "shh_addSymKey")]
		fn add_sym_key(&self, types::Symmetric) -> Result<types::Identity, Error>;

		/// Get public key. Succeeds if identity is stored and asymmetric.
		#[rpc(name = "shh_getPublicKey")]
		fn get_public(&self, types::Identity) -> Result<types::Public, Error>;

		/// Get private key. Succeeds if identity is stored and asymmetric.
		#[rpc(name = "shh_getPrivateKey")]
		fn get_private(&self, types::Identity) -> Result<types::Private, Error>;

		#[rpc(name = "shh_getSymKey")]
		fn get_symmetric(&self, types::Identity) -> Result<types::Symmetric, Error>;

		/// Delete key pair denoted by given identity.
		///
		/// Return true if successfully removed, false if unknown,
		/// and error otherwise.
		#[rpc(name = "shh_deleteKey")]
		fn remove_key(&self, types::Identity) -> Result<bool, Error>;

		/// Post a message to the network with given parameters.
		#[rpc(name = "shh_post")]
		fn post(&self, types::PostRequest) -> Result<bool, Error>;

		/// Create a new polled filter.
		#[rpc(name = "shh_newMessageFilter")]
		fn new_filter(&self, types::FilterRequest) -> Result<types::Identity, Error>;

		/// Poll changes on a polled filter.
		#[rpc(name = "shh_getFilterMessages")]
		fn poll_changes(&self, types::Identity) -> Result<Vec<types::FilterItem>, Error>;

		/// Delete polled filter. Return bool indicating success.
		#[rpc(name = "shh_deleteMessageFilter")]
		fn delete_filter(&self, types::Identity) -> Result<bool, Error>;
	}
}

build_rpc_trait! {
	/// Whisper RPC pubsub.
	pub trait WhisperPubSub {
		type Metadata;

		#[pubsub(name = "hello")] {
			/// Subscribe to messages matching the filter.
			#[rpc(name = "ssh_subscribe")]
			fn subscribe(&self, Self::Metadata, pubsub::Subscriber<types::FilterItem>, types::FilterRequest);

			/// Unsubscribe from filter matching given ID. Return
			/// true on success, error otherwise.
			#[rpc(name = "shh_unsubscribe")]
			fn unsubscribe(&self, SubscriptionId) -> BoxFuture<bool, Error>;
		}
	}
}

/// Something which can send messages to the network.
pub trait MessageSender: Send + Sync {
	/// Give message to the whisper network for relay.
	fn relay(&self, message: Message);
}

impl MessageSender for ::net::MessagePoster {
	fn relay(&self, message: Message) {
		self.post_message(message)
	}
}

/// Default, simple metadata implementation.
#[derive(Clone, Default)]
pub struct Meta {
	session: Option<Arc<Session>>,
}

impl Metadata for Meta {}
impl PubSubMetadata for Meta {
	fn session(&self) -> Option<Arc<Session>> {
		self.session.clone()
	}
}

/// Implementation of whisper RPC.
pub struct WhisperClient<S, M = Meta> {
	store: RwLock<key_store::KeyStore>,
	sender: S,
	filter_manager: Arc<filter::Manager>,
	filter_ids_rng: Mutex<XorShiftRng>,
	_meta: ::std::marker::PhantomData<M>,
}

impl<S> WhisperClient<S> {
	/// Create a new whisper client with basic metadata.
	///
	/// This spawns a thread for handling
	/// asynchronous work like performing PoW on messages or handling
	/// subscriptions.
	pub fn with_simple_meta(sender: S, filter_manager: Arc<filter::Manager>)
		-> ::std::io::Result<Self>
	{
		WhisperClient::new(sender, filter_manager)
	}
}

impl<S, M> WhisperClient<S, M> {
	/// Create a new whisper client.
	///
	/// This spawns a thread for handling
	/// asynchronous work like performing PoW on messages or handling
	/// subscriptions.
	pub fn new(sender: S, filter_manager: Arc<filter::Manager>) -> ::std::io::Result<Self> {
		let filter_ids_rng = {
			let mut rng = ::rand::thread_rng();
			XorShiftRng::from_seed(rng.gen())
		};

		Ok(WhisperClient {
			store: RwLock::new(KeyStore::new()?),
			sender: sender,
			filter_manager: filter_manager,
			filter_ids_rng: Mutex::new(filter_ids_rng),
			_meta: ::std::marker::PhantomData,
		})
	}

	fn delete_filter_kind(&self, id: H256, kind: filter::Kind) -> bool {
		match self.filter_manager.kind(&id) {
			Some(k) if k == kind => {
				self.filter_manager.remove(&id);
				true
			}
			None | Some(_) => false,
		}
	}
}

impl<S: MessageSender + 'static, M: Send + Sync + 'static> Whisper for WhisperClient<S, M> {
	fn new_key_pair(&self) -> Result<types::Identity, Error> {
		let mut store = self.store.write();
		let key_pair = Key::new_asymmetric(store.rng());

		Ok(HexEncode(store.insert(key_pair)))
	}

	fn add_private_key(&self, private: types::Private) -> Result<types::Identity, Error> {
		let key_pair = Key::from_secret(private.into_inner().into())
			.map_err(|_| whisper_error("Invalid private key"))?;

		Ok(HexEncode(self.store.write().insert(key_pair)))
	}

	fn new_sym_key(&self) -> Result<types::Identity, Error> {
		let mut store = self.store.write();
		let key = Key::new_symmetric(store.rng());

		Ok(HexEncode(store.insert(key)))
	}

	fn add_sym_key(&self, raw_key: types::Symmetric) -> Result<types::Identity, Error> {
		let raw_key = raw_key.into_inner().0;
		let key = Key::from_raw_symmetric(raw_key);

		Ok(HexEncode(self.store.write().insert(key)))
	}

	fn get_public(&self, id: types::Identity) -> Result<types::Public, Error> {
		self.store.read().public(&id.into_inner())
			.cloned()
			.map(HexEncode)
			.ok_or_else(|| whisper_error("Unknown identity"))
	}

	fn get_private(&self, id: types::Identity) -> Result<types::Private, Error> {
		self.store.read().secret(&id.into_inner())
			.map(|x| (&**x).clone())
			.map(HexEncode)
			.ok_or_else(|| whisper_error("Unknown identity"))
	}

	fn get_symmetric(&self, id: types::Identity) -> Result<types::Symmetric, Error> {
		self.store.read().symmetric(&id.into_inner())
			.cloned()
			.map(H256)
			.map(HexEncode)
			.ok_or_else(|| whisper_error("Unknown identity"))
	}

	fn remove_key(&self, id: types::Identity) -> Result<bool, Error> {
		Ok(self.store.write().remove(&id.into_inner()))
	}

	fn post(&self, req: types::PostRequest) -> Result<bool, Error> {
		use self::crypto::EncryptionInstance;

		let encryption = match req.to {
			types::Receiver::Public(public) => EncryptionInstance::ecies(public.into_inner())
				.map_err(whisper_error)?,
			types::Receiver::Identity(id) => self.store.read().encryption_instance(&id.into_inner())
				.map_err(whisper_error)?,
		};

		let sign_with = match req.from {
			Some(from) => {
				Some(
					self.store.read().secret(&from.into_inner())
						.cloned()
						.ok_or_else(|| whisper_error("Unknown identity `from`"))?
				)
			}
			None => None,
		};

		let encrypted = {
			let payload = payload::encode(payload::EncodeParams {
				message: &req.payload.into_inner(),
				padding: req.padding.map(|p| p.into_inner()).as_ref().map(|x| &x[..]),
				sign_with: sign_with.as_ref(),
			}).map_err(whisper_error)?;

			encryption.encrypt(&payload)
		};

		// mining the packet is the heaviest item of work by far.
		// there may be a benefit to dispatching this onto the CPU pool
		// and returning a future. but then things get _less_ efficient
		//
		// if the server infrastructure has more threads than the CPU pool.
		let message = Message::create(CreateParams {
			ttl: req.ttl,
			payload: encrypted,
			topics: req.topics.into_iter().map(|x| abridge_topic(&x.into_inner())).collect(),
			work: req.priority,
		});

		self.sender.relay(message);

		Ok(true)
	}

	fn new_filter(&self, req: types::FilterRequest) -> Result<types::Identity, Error> {
		let filter = Filter::new(req).map_err(whisper_error)?;
		let id = self.filter_ids_rng.lock().gen();

		self.filter_manager.insert_polled(id, filter);
		Ok(HexEncode(id))
	}

	fn poll_changes(&self, id: types::Identity) -> Result<Vec<types::FilterItem>, Error> {
		match self.filter_manager.poll_changes(&id.into_inner()) {
			None => Err(whisper_error("no such message filter")),
			Some(items) => Ok(items),
		}
	}

	fn delete_filter(&self, id: types::Identity) -> Result<bool, Error> {
		Ok(self.delete_filter_kind(id.into_inner(), filter::Kind::Poll))
	}
}

impl<S: MessageSender + 'static, M: Send + Sync + PubSubMetadata> WhisperPubSub for WhisperClient<S, M> {
	type Metadata = M;

	fn subscribe(
		&self,
		_meta: Self::Metadata,
		subscriber: pubsub::Subscriber<types::FilterItem>,
		req: types::FilterRequest,
	) {
		match Filter::new(req) {
			Ok(filter) => {
				let id: H256 = self.filter_ids_rng.lock().gen();

				if let Ok(sink) = subscriber.assign_id(SubscriptionId::String(id.hex())) {
					self.filter_manager.insert_subscription(id, filter, sink);
				}
			}
			Err(reason) => { let _ = subscriber.reject(whisper_error(reason)); }
		}
	}

	fn unsubscribe(&self, id: SubscriptionId) -> BoxFuture<bool, Error> {
		use std::str::FromStr;

		let res = match id {
			SubscriptionId::String(s) => H256::from_str(&s)
				.map_err(|_| "unrecognized ID")
				.map(|id| self.delete_filter_kind(id, filter::Kind::Subscription)),
			SubscriptionId::Number(_) => Err("unrecognized ID"),
		};

		Box::new(future::done(res.map_err(whisper_error)))
	}
}
