#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use core::marker::PhantomData;
use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};

pub trait WeightInfo {
    fn declare_presence() -> Weight;
    fn declare_presence_with_commitment() -> Weight;
    fn vote_presence() -> Weight;
    fn finalize_presence() -> Weight;
    fn slash_presence() -> Weight;
    fn set_quorum_config() -> Weight;
    fn set_validator_status() -> Weight;
    fn set_epoch_active() -> Weight;
    fn reveal_commitment() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn declare_presence() -> Weight {
        Weight::from_parts(25_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(2))
    }

    fn declare_presence_with_commitment() -> Weight {
        Weight::from_parts(35_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(3))
    }

    fn vote_presence() -> Weight {
        Weight::from_parts(40_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(4))
            .saturating_add(T::DbWeight::get().writes(3))
    }

    fn finalize_presence() -> Weight {
        Weight::from_parts(20_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn slash_presence() -> Weight {
        Weight::from_parts(15_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn set_quorum_config() -> Weight {
        Weight::from_parts(10_000_000, 0).saturating_add(T::DbWeight::get().writes(1))
    }

    fn set_validator_status() -> Weight {
        Weight::from_parts(10_000_000, 0).saturating_add(T::DbWeight::get().writes(1))
    }

    fn set_epoch_active() -> Weight {
        Weight::from_parts(12_000_000, 0).saturating_add(T::DbWeight::get().writes(2))
    }

    fn reveal_commitment() -> Weight {
        Weight::from_parts(40_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(2))
    }
}

impl WeightInfo for () {
    fn declare_presence() -> Weight {
        Weight::from_parts(25_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2))
            .saturating_add(RocksDbWeight::get().writes(2))
    }

    fn declare_presence_with_commitment() -> Weight {
        Weight::from_parts(35_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2))
            .saturating_add(RocksDbWeight::get().writes(3))
    }

    fn vote_presence() -> Weight {
        Weight::from_parts(40_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(4))
            .saturating_add(RocksDbWeight::get().writes(3))
    }

    fn finalize_presence() -> Weight {
        Weight::from_parts(20_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2))
            .saturating_add(RocksDbWeight::get().writes(1))
    }

    fn slash_presence() -> Weight {
        Weight::from_parts(15_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1))
            .saturating_add(RocksDbWeight::get().writes(1))
    }

    fn set_quorum_config() -> Weight {
        Weight::from_parts(10_000_000, 0).saturating_add(RocksDbWeight::get().writes(1))
    }

    fn set_validator_status() -> Weight {
        Weight::from_parts(10_000_000, 0).saturating_add(RocksDbWeight::get().writes(1))
    }

    fn set_epoch_active() -> Weight {
        Weight::from_parts(12_000_000, 0).saturating_add(RocksDbWeight::get().writes(2))
    }

    fn reveal_commitment() -> Weight {
        Weight::from_parts(40_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(3))
            .saturating_add(RocksDbWeight::get().writes(2))
    }
}
