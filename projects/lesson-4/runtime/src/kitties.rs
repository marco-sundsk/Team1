use support::{decl_module, decl_storage, ensure, StorageValue, StorageMap, dispatch::Result, Parameter};
use sr_primitives::traits::{SimpleArithmetic, Bounded};
use codec::{Encode, Decode};
use runtime_io::blake2_128;
use system::ensure_signed;
use rstd::result;

pub trait Trait: system::Trait {
	type KittyIndex: Parameter + SimpleArithmetic + Bounded + Default + Copy;
}

#[derive(Encode, Decode)]
pub struct Kitty(pub [u8; 16]);

decl_storage! {
	trait Store for Module<T: Trait> as Kitties {
		/// Stores all the kitties, key is the kitty id / index
		pub Kitties get(kitty): map T::KittyIndex => Option<Kitty>;
		/// Stores the total number of kitties. i.e. the next kitty index
		pub KittiesCount get(kitties_count): T::KittyIndex;
		/// Get user kitty index by kitty id, cause a kitty only have one owner, so can be done like this.
		pub OwnedKittiesIndex get(owned_kitties_index): map T::KittyIndex  => T::KittyIndex;

		/// Get kitty ID by account ID and user kitty index
		pub OwnedKitties get(owned_kitties): map (T::AccountId, T::KittyIndex) => T::KittyIndex;
		/// Get number of kitties by account ID
		pub OwnedKittiesCount get(owned_kitties_count): map T::AccountId => T::KittyIndex;
		
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		/// Create a new kitty
		pub fn create(origin) {
			let sender = ensure_signed(origin)?;

			// 作业：重构create方法，避免重复代码
			Self::insert_kitty(sender.clone(), Self::next_kitty_id()?, Kitty(Self::random_value(&sender)));

		}

		/// Breed kitties
		pub fn breed(origin, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) {
			let sender = ensure_signed(origin)?;

			Self::do_breed(sender, kitty_id_1, kitty_id_2)?;
		}

		/// Transfer kitties
		fn transfer(origin, to: T::AccountId, kitty_id: T::KittyIndex) -> Result {
            let sender = ensure_signed(origin)?;

            // let owner = Self::owner_of(kitty_id).ok_or("No owner for this kitty")?;
            // ensure!(owner == sender, "You do not own this kitty");

            Self::transfer_from(sender, to, kitty_id)?;

            Ok(())
        }
	}
}

fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
	// 作业：实现combine_dna
	// 伪代码：
	// selector.map_bits(|bit, index| if (bit == 1) { dna1 & (1 << index) } else { dna2 & (1 << index) })
	// 注意 map_bits这个方法不存在。只要能达到同样效果，不局限算法
	// 测试数据：dna1 = 0b11110000, dna2 = 0b11001100, selector = 0b10101010, 返回值 0b11100100
	let mut res = 0b00000000;
	for idx in [1, 2, 4, 8, 16, 32, 64, 128].iter() {
		if selector&idx != 0 {  // using dna1
			res = res|(dna1&idx);
		} else {  // using dna2
			res = res|(dna2&idx);
		}
	}
	res
}

impl<T: Trait> Module<T> {
	fn random_value(sender: &T::AccountId) -> [u8; 16] {
		let payload = (<system::Module<T>>::random_seed(), sender, <system::Module<T>>::extrinsic_index(), <system::Module<T>>::block_number());
		payload.using_encoded(blake2_128)
	}

	fn next_kitty_id() -> result::Result<T::KittyIndex, &'static str> {
		let kitty_id = Self::kitties_count();
		if kitty_id == T::KittyIndex::max_value() {
			return Err("Kitties count overflow");
		}
		Ok(kitty_id)
	}

	fn insert_kitty(owner: T::AccountId, kitty_id: T::KittyIndex, kitty: Kitty) {
		// Create and store kitty
		<Kitties<T>>::insert(kitty_id, kitty);
		<KittiesCount<T>>::put(kitty_id + 1.into());

		// Store the ownership information
		let user_kitties_id = Self::owned_kitties_count(owner.clone());
		<OwnedKitties<T>>::insert((owner.clone(), user_kitties_id), kitty_id);
		<OwnedKittiesCount<T>>::insert(owner, user_kitties_id + 1.into());

		// update user kitty index
		<OwnedKittiesIndex<T>>::insert(kitty_id, user_kitties_id);
	}

	fn do_breed(sender: T::AccountId, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) -> Result {
		let kitty1 = Self::kitty(kitty_id_1);
		let kitty2 = Self::kitty(kitty_id_2);

		ensure!(kitty1.is_some(), "Invalid kitty_id_1");
		ensure!(kitty2.is_some(), "Invalid kitty_id_2");
		ensure!(kitty_id_1 != kitty_id_2, "Needs different parent");

		let kitty_id = Self::next_kitty_id()?;

		let kitty1_dna = kitty1.unwrap().0;
		let kitty2_dna = kitty2.unwrap().0;

		// Generate a random 128bit value
		let selector = Self::random_value(&sender);
		let mut new_dna = [0u8; 16];

		// Combine parents and selector to create new kitty
		for i in 0..kitty1_dna.len() {
			new_dna[i] = combine_dna(kitty1_dna[i], kitty2_dna[i], selector[i]);
		}

		Self::insert_kitty(sender, kitty_id, Kitty(new_dna));

		Ok(())
	}

	fn transfer_from(from: T::AccountId, to: T::AccountId, kitty_id: T::KittyIndex) -> Result {
        // let owner = Self::owner_of(kitty_id).ok_or("No owner for this kitty")?;
		let kitty = Self::kitty(kitty_id);

		ensure!(kitty.is_some(), "Invalid kitty_id");

        // ensure!(owner == from, "'from' account does not own this kitty");

        let owned_kitty_count_from = Self::owned_kitties_count(from.clone());
        let owned_kitty_count_to = Self::owned_kitties_count(to.clone());

        let new_owned_kitty_count_to = owned_kitty_count_to + 1.into();

        let new_owned_kitty_count_from = owned_kitty_count_from - 1.into();

        // let kitty_index = <OwnedKittiesIndex<T>>::get(kitty_id);
		let kitty_index = Self::owned_kitties_index(kitty_id);
        if kitty_index != new_owned_kitty_count_from {
			// there two way to get from storage map:
            // let last_kitty_id: T::KittyIndex = <OwnedKitties<T>>::get((from.clone(), new_owned_kitty_count_from));
			let last_kitty_id: T::KittyIndex = Self::owned_kitties((from.clone(), new_owned_kitty_count_from));
            <OwnedKitties<T>>::insert((from.clone(), kitty_index), last_kitty_id);
            <OwnedKitties<T>>::insert((from.clone(), new_owned_kitty_count_from), kitty_index);
        }

		// add kitty to newer owner
		let kitty_index_to = owned_kitty_count_to;
		<OwnedKitties<T>>::insert((to.clone(), kitty_index_to), kitty_id);
		<OwnedKittiesCount<T>>::insert(to.clone(), new_owned_kitty_count_to);
		
		// remove kitty from old owner
		<OwnedKitties<T>>::remove((from.clone(), new_owned_kitty_count_from));
		<OwnedKittiesCount<T>>::insert(from.clone(), new_owned_kitty_count_from);

		// update user kitty index
		<OwnedKittiesIndex<T>>::insert(kitty_id, kitty_index_to);

        // Self::deposit_event(RawEvent::Transferred(from, to, kitty_id));

        Ok(())
    }
}
