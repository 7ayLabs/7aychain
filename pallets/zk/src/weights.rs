#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn verify_share_proof() -> Weight;
    fn verify_presence_proof() -> Weight;
    fn verify_access_proof() -> Weight;
    fn add_trusted_verifier() -> Weight;
    fn remove_trusted_verifier() -> Weight;
    fn consume_nullifier() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn verify_share_proof() -> Weight {
        Weight::from_parts(50_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(4))
    }

    fn verify_presence_proof() -> Weight {
        Weight::from_parts(60_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(4))
            .saturating_add(T::DbWeight::get().writes(5))
    }

    fn verify_access_proof() -> Weight {
        Weight::from_parts(55_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(4))
    }

    fn add_trusted_verifier() -> Weight {
        Weight::from_parts(15_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn remove_trusted_verifier() -> Weight {
        Weight::from_parts(15_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn consume_nullifier() -> Weight {
        Weight::from_parts(25_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(1))
    }
}

impl WeightInfo for () {
    fn verify_share_proof() -> Weight {
        Weight::from_parts(50_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(3))
            .saturating_add(RocksDbWeight::get().writes(4))
    }

    fn verify_presence_proof() -> Weight {
        Weight::from_parts(60_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(4))
            .saturating_add(RocksDbWeight::get().writes(5))
    }

    fn verify_access_proof() -> Weight {
        Weight::from_parts(55_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(3))
            .saturating_add(RocksDbWeight::get().writes(4))
    }

    fn add_trusted_verifier() -> Weight {
        Weight::from_parts(15_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1))
            .saturating_add(RocksDbWeight::get().writes(1))
    }

    fn remove_trusted_verifier() -> Weight {
        Weight::from_parts(15_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1))
            .saturating_add(RocksDbWeight::get().writes(1))
    }

    fn consume_nullifier() -> Weight {
        Weight::from_parts(25_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2))
            .saturating_add(RocksDbWeight::get().writes(1))
    }
}
