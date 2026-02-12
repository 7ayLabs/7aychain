#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use core::marker::PhantomData;
use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};

pub trait WeightInfo {
    fn process_scan_data(n: u32) -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn process_scan_data(n: u32) -> Weight {
        Weight::from_parts(10_000_000, 0)
            .saturating_add(Weight::from_parts(5_000_000, 0).saturating_mul(n as u64))
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().reads(n as u64))
            .saturating_add(T::DbWeight::get().writes(n as u64 + 2))
    }
}

impl WeightInfo for () {
    fn process_scan_data(n: u32) -> Weight {
        Weight::from_parts(10_000_000, 0)
            .saturating_add(Weight::from_parts(5_000_000, 0).saturating_mul(n as u64))
            .saturating_add(RocksDbWeight::get().reads(1))
            .saturating_add(RocksDbWeight::get().reads(n as u64))
            .saturating_add(RocksDbWeight::get().writes(n as u64 + 2))
    }
}
