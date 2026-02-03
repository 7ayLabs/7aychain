#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn grant_capability() -> Weight;
    fn revoke_capability() -> Weight;
    fn delegate_capability() -> Weight;
    fn update_capability() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn grant_capability() -> Weight {
        Weight::from_parts(40_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(4))
    }

    fn revoke_capability() -> Weight {
        Weight::from_parts(35_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(2))
    }

    fn delegate_capability() -> Weight {
        Weight::from_parts(50_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(5))
    }

    fn update_capability() -> Weight {
        Weight::from_parts(30_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
}

impl WeightInfo for () {
    fn grant_capability() -> Weight {
        Weight::from_parts(40_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2))
            .saturating_add(RocksDbWeight::get().writes(4))
    }

    fn revoke_capability() -> Weight {
        Weight::from_parts(35_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2))
            .saturating_add(RocksDbWeight::get().writes(2))
    }

    fn delegate_capability() -> Weight {
        Weight::from_parts(50_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(3))
            .saturating_add(RocksDbWeight::get().writes(5))
    }

    fn update_capability() -> Weight {
        Weight::from_parts(30_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1))
            .saturating_add(RocksDbWeight::get().writes(1))
    }
}
