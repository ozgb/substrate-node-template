// We make sure this pallet uses `no_std` for compiling to Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use pallet_timestamp::{self as timestamp};

#[frame_support::pallet]
pub mod pallet {
	// TODO: Remove the hard upper limit of number of input txs
	// i.e. someone should be able to receive unlimited txs
	const MAX_INPUTS: u32 = 100;
	type TxId = [u8; 16];
	const NULL_ID: TxId = [0u8; 16];
	use super::timestamp;
	use frame_support::{pallet_prelude::*, sp_runtime::DispatchResult, StorageHasher};
	use frame_system::pallet_prelude::*;

	#[derive(Clone, Encode, Decode, PartialEq, Copy, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct UtxoTransaction<T: Config> {
		id: TxId,
		owner: T::AccountId,
		// TODO: Change this to a variable sized structure
		inputs: [TxId; MAX_INPUTS as usize],
		balance: u64,
	}

	//#[pallet::storage]
	//pub type OutputsConsumed<T: Config> = StorageMap<
	//	_,
	//	Twox64Concat,
	//	TxId,
	//	UtxoTransaction<T>,
	//>;

	//#[pallet::storage]
	//pub type UnspentOutputOwners<T: Config> = StorageMap<
	//	_,
	//	Twox64Concat,
	//	T::AccountId,
	//	TxId
	//>;

	#[pallet::storage]
	pub type UnspentOutputs<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<UtxoTransaction<T>, ConstU32<MAX_INPUTS>>,
	>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + timestamp::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		#[pallet::constant]
		type AirdropAmount: Get<u64>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// Transaction processed successfully
		TransactionSuccessful { id: TxId },
	}

	#[pallet::error]
	pub enum Error<T> {
		// User has reached their tx input limit
		InputLimitReached,
	}

	// Pallet internal functions
	impl<T: Config> Pallet<T> {
		pub fn add_tx(tx: UtxoTransaction<T>) -> DispatchResult {
			let owner = tx.owner.clone();
			UnspentOutputs::<T>::mutate(owner, |v| match v {
				None => {
					*v = Some(BoundedVec::new());
					Ok(())
				},
				Some(v) => v.try_push(tx).map_err(|_| Error::<T>::InputLimitReached),
			})?;

			Ok(())
		}

		pub fn gen_tx_id(
			owner: T::AccountId,
			inputs: Vec<TxId>,
			balance: u64,
			now: <T as timestamp::Config>::Moment,
		) -> TxId {
			let input = [
				owner.encode().as_slice(),
				inputs.encode().as_slice(),
				balance.encode().as_slice(),
				now.encode().as_slice(),
			]
			.concat();
			Blake2_128::hash(&input)
		}

		pub fn do_airdrop_tx(
			to: T::AccountId,
			now: <T as timestamp::Config>::Moment,
		) -> Result<(), DispatchError> {
			let balance = T::AirdropAmount::get();
			let id = Self::gen_tx_id(to.clone(), vec![], balance, now);

			// Create airdrop tx
			let new_tx =
				UtxoTransaction { id, owner: to, inputs: [NULL_ID; MAX_INPUTS as usize], balance };

			Self::add_tx(new_tx)?;

			Ok(())
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn airdrop_tx(origin: OriginFor<T>, to: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;

			let now = timestamp::Pallet::<T>::get();
			Self::do_airdrop_tx(to, now)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn get_time(origin: OriginFor<T>) -> DispatchResult {
			let _sender = ensure_signed(origin)?;
			let _now = timestamp::Pallet::<T>::get();
			Ok(())
		}
	}
}
