#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::expect_used)]
extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, Get, ReservableCurrency, StorageVersion},
    };
    use frame_system::pallet_prelude::*;
    use seveny_primitives::{
        constants::{
            EVIDENCE_REWARD_MAX, MAX_STAKE_RATIO, MIN_VALIDATORS, SLASH_CRITICAL, SLASH_MINOR,
            SLASH_MODERATE, SLASH_SEVERE,
        },
        types::{ValidatorId, ViolationType},
    };
    use sp_arithmetic::Perbill;
    use sp_runtime::{traits::Zero, Saturating};
    use alloc::vec::Vec;

    use crate::WeightInfo;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
        type WeightInfo: WeightInfo;
        type Currency: ReservableCurrency<Self::AccountId>;

        #[pallet::constant]
        type MinStake: Get<BalanceOf<Self>>;

        #[pallet::constant]
        type MaxValidators: Get<u32>;

        #[pallet::constant]
        type BondingDuration: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type SlashDeferDuration: Get<BlockNumberFor<Self>>;
    }

    #[derive(Clone, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub enum ValidatorStatus {
        Bonding,
        Active,
        Unbonding,
        Slashed,
    }

    #[derive(Clone, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    #[scale_info(skip_type_params(T))]
    pub struct ValidatorInfo<T: Config> {
        pub id: ValidatorId,
        pub controller: T::AccountId,
        pub stake: BalanceOf<T>,
        pub status: ValidatorStatus,
        pub registered_at: BlockNumberFor<T>,
        pub unbonding_at: Option<BlockNumberFor<T>>,
    }

    #[derive(Clone, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    #[scale_info(skip_type_params(T))]
    pub struct SlashRecord<T: Config> {
        pub validator: ValidatorId,
        pub amount: BalanceOf<T>,
        pub violation: ViolationType,
        pub block: BlockNumberFor<T>,
        pub applied: bool,
    }

    #[pallet::storage]
    #[pallet::getter(fn validators)]
    pub type Validators<T: Config> =
        StorageMap<_, Blake2_128Concat, ValidatorId, ValidatorInfo<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn validator_stake)]
    pub type ValidatorStake<T: Config> =
        StorageMap<_, Blake2_128Concat, ValidatorId, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn total_stake)]
    pub type TotalStake<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn validator_count)]
    pub type ValidatorCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn active_validator_count)]
    pub type ActiveValidatorCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn validator_by_controller)]
    pub type ValidatorByController<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, ValidatorId, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pending_slashes)]
    pub type PendingSlashes<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, SlashRecord<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn slash_count)]
    pub type SlashCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ValidatorRegistered {
            validator: ValidatorId,
            controller: T::AccountId,
            stake: BalanceOf<T>,
        },
        ValidatorActivated {
            validator: ValidatorId,
        },
        ValidatorDeactivated {
            validator: ValidatorId,
        },
        StakeIncreased {
            validator: ValidatorId,
            additional: BalanceOf<T>,
            total: BalanceOf<T>,
        },
        StakeDecreased {
            validator: ValidatorId,
            removed: BalanceOf<T>,
            total: BalanceOf<T>,
        },
        UnbondingStarted {
            validator: ValidatorId,
            unbond_at: BlockNumberFor<T>,
        },
        ValidatorSlashed {
            validator: ValidatorId,
            amount: BalanceOf<T>,
            violation: ViolationType,
        },
        SlashDeferred {
            validator: ValidatorId,
            amount: BalanceOf<T>,
            defer_until: BlockNumberFor<T>,
        },
        SlashApplied {
            validator: ValidatorId,
            amount: BalanceOf<T>,
        },
        EvidenceRewardPaid {
            reporter: T::AccountId,
            amount: BalanceOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        ValidatorAlreadyRegistered,
        ValidatorNotFound,
        InsufficientStake,
        StakeTooHigh,
        MinValidatorsRequired,
        MaxValidatorsReached,
        AlreadyActive,
        NotActive,
        AlreadyUnbonding,
        BondingPeriodNotElapsed,
        UnbondingPeriodNotElapsed,
        InvalidViolationType,
        SlashNotFound,
        SlashAlreadyApplied,
        ArithmeticOverflow,
        InsufficientBalance,
        ControllerAlreadyUsed,
    }

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub initial_validators: Vec<(T::AccountId, BalanceOf<T>)>,
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            use sp_runtime::traits::Hash;

            for (controller, stake) in &self.initial_validators {
                let hash = T::Hashing::hash_of(controller);
                let validator_id =
                    ValidatorId::from(sp_core::H256(hash.as_ref().try_into().unwrap_or([0u8; 32])));

                let info = ValidatorInfo {
                    id: validator_id,
                    controller: controller.clone(),
                    stake: *stake,
                    status: ValidatorStatus::Active,
                    registered_at: BlockNumberFor::<T>::zero(),
                    unbonding_at: None,
                };

                Validators::<T>::insert(validator_id, info);
                ValidatorStake::<T>::insert(validator_id, stake);
                ValidatorByController::<T>::insert(controller, validator_id);
                TotalStake::<T>::mutate(|total| {
                    *total = total.saturating_add(*stake);
                });
                ValidatorCount::<T>::mutate(|count| {
                    *count = count.saturating_add(1);
                });
                ActiveValidatorCount::<T>::mutate(|count| {
                    *count = count.saturating_add(1);
                });
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_validator())]
        pub fn register_validator(origin: OriginFor<T>, stake: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            ensure!(
                ValidatorByController::<T>::get(&who).is_none(),
                Error::<T>::ControllerAlreadyUsed
            );
            ensure!(stake >= T::MinStake::get(), Error::<T>::InsufficientStake);
            ensure!(
                ValidatorCount::<T>::get() < T::MaxValidators::get(),
                Error::<T>::MaxValidatorsReached
            );

            Self::ensure_stake_ratio_valid(stake)?;

            T::Currency::reserve(&who, stake)?;

            let validator_id = Self::account_to_validator(&who);

            let info = ValidatorInfo {
                id: validator_id,
                controller: who.clone(),
                stake,
                status: ValidatorStatus::Bonding,
                registered_at: block_number,
                unbonding_at: None,
            };

            Validators::<T>::insert(validator_id, info);
            ValidatorStake::<T>::insert(validator_id, stake);
            ValidatorByController::<T>::insert(&who, validator_id);
            TotalStake::<T>::mutate(|total| {
                *total = total.saturating_add(stake);
            });
            ValidatorCount::<T>::mutate(|count| {
                *count = count.saturating_add(1);
            });

            Self::deposit_event(Event::ValidatorRegistered {
                validator: validator_id,
                controller: who,
                stake,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::activate_validator())]
        pub fn activate_validator(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let validator_id =
                ValidatorByController::<T>::get(&who).ok_or(Error::<T>::ValidatorNotFound)?;
            let mut info = Validators::<T>::get(validator_id).ok_or(Error::<T>::ValidatorNotFound)?;

            ensure!(
                info.status == ValidatorStatus::Bonding,
                Error::<T>::AlreadyActive
            );

            let bonding_end = info.registered_at.saturating_add(T::BondingDuration::get());
            ensure!(
                block_number >= bonding_end,
                Error::<T>::BondingPeriodNotElapsed
            );

            info.status = ValidatorStatus::Active;
            Validators::<T>::insert(validator_id, info);

            ActiveValidatorCount::<T>::mutate(|count| {
                *count = count.saturating_add(1);
            });

            Self::deposit_event(Event::ValidatorActivated {
                validator: validator_id,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::deactivate_validator())]
        pub fn deactivate_validator(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let validator_id =
                ValidatorByController::<T>::get(&who).ok_or(Error::<T>::ValidatorNotFound)?;
            let mut info = Validators::<T>::get(validator_id).ok_or(Error::<T>::ValidatorNotFound)?;

            ensure!(
                info.status == ValidatorStatus::Active,
                Error::<T>::NotActive
            );

            let active_count = ActiveValidatorCount::<T>::get();
            ensure!(
                active_count > MIN_VALIDATORS,
                Error::<T>::MinValidatorsRequired
            );

            info.status = ValidatorStatus::Unbonding;
            info.unbonding_at = Some(block_number);
            Validators::<T>::insert(validator_id, info);

            ActiveValidatorCount::<T>::mutate(|count| {
                *count = count.saturating_sub(1);
            });

            let unbond_at = block_number.saturating_add(T::BondingDuration::get());

            Self::deposit_event(Event::ValidatorDeactivated {
                validator: validator_id,
            });
            Self::deposit_event(Event::UnbondingStarted {
                validator: validator_id,
                unbond_at,
            });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::withdraw_stake())]
        pub fn withdraw_stake(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let validator_id =
                ValidatorByController::<T>::get(&who).ok_or(Error::<T>::ValidatorNotFound)?;
            let info = Validators::<T>::get(validator_id).ok_or(Error::<T>::ValidatorNotFound)?;

            ensure!(
                info.status == ValidatorStatus::Unbonding,
                Error::<T>::NotActive
            );

            let unbonding_at = info.unbonding_at.ok_or(Error::<T>::NotActive)?;
            let unbond_end = unbonding_at.saturating_add(T::BondingDuration::get());
            ensure!(
                block_number >= unbond_end,
                Error::<T>::UnbondingPeriodNotElapsed
            );

            let stake = ValidatorStake::<T>::get(validator_id);
            T::Currency::unreserve(&who, stake);

            Validators::<T>::remove(validator_id);
            ValidatorStake::<T>::remove(validator_id);
            ValidatorByController::<T>::remove(&who);
            TotalStake::<T>::mutate(|total| {
                *total = total.saturating_sub(stake);
            });
            ValidatorCount::<T>::mutate(|count| {
                *count = count.saturating_sub(1);
            });

            Self::deposit_event(Event::StakeDecreased {
                validator: validator_id,
                removed: stake,
                total: BalanceOf::<T>::zero(),
            });

            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::increase_stake())]
        pub fn increase_stake(origin: OriginFor<T>, additional: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let validator_id =
                ValidatorByController::<T>::get(&who).ok_or(Error::<T>::ValidatorNotFound)?;
            let mut info = Validators::<T>::get(validator_id).ok_or(Error::<T>::ValidatorNotFound)?;

            let new_stake = info.stake.saturating_add(additional);
            Self::ensure_stake_ratio_valid(new_stake)?;

            T::Currency::reserve(&who, additional)?;

            info.stake = new_stake;
            Validators::<T>::insert(validator_id, info);
            ValidatorStake::<T>::insert(validator_id, new_stake);
            TotalStake::<T>::mutate(|total| {
                *total = total.saturating_add(additional);
            });

            Self::deposit_event(Event::StakeIncreased {
                validator: validator_id,
                additional,
                total: new_stake,
            });

            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::slash_validator())]
        pub fn slash_validator(
            origin: OriginFor<T>,
            validator: ValidatorId,
            violation: ViolationType,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let info = Validators::<T>::get(validator).ok_or(Error::<T>::ValidatorNotFound)?;
            let stake = ValidatorStake::<T>::get(validator);

            let slash_pct = Self::get_slash_percentage(&violation);
            let slash_amount = slash_pct.mul_floor(stake);

            let slash_id = SlashCount::<T>::get();
            SlashCount::<T>::put(slash_id.saturating_add(1));

            let defer_until = block_number.saturating_add(T::SlashDeferDuration::get());

            let slash_record = SlashRecord {
                validator,
                amount: slash_amount,
                violation: violation.clone(),
                block: block_number,
                applied: false,
            };

            PendingSlashes::<T>::insert(slash_id, slash_record);

            Self::deposit_event(Event::SlashDeferred {
                validator,
                amount: slash_amount,
                defer_until,
            });

            if violation == ViolationType::Critical {
                let mut info_mut = info;
                info_mut.status = ValidatorStatus::Slashed;
                Validators::<T>::insert(validator, info_mut);

                if ActiveValidatorCount::<T>::get() > 0 {
                    ActiveValidatorCount::<T>::mutate(|count| {
                        *count = count.saturating_sub(1);
                    });
                }
            }

            Self::deposit_event(Event::ValidatorSlashed {
                validator,
                amount: slash_amount,
                violation,
            });

            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::apply_slash())]
        pub fn apply_slash(origin: OriginFor<T>, slash_id: u64) -> DispatchResult {
            ensure_root(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let mut slash_record =
                PendingSlashes::<T>::get(slash_id).ok_or(Error::<T>::SlashNotFound)?;

            ensure!(!slash_record.applied, Error::<T>::SlashAlreadyApplied);

            let defer_until = slash_record
                .block
                .saturating_add(T::SlashDeferDuration::get());
            ensure!(
                block_number >= defer_until,
                Error::<T>::UnbondingPeriodNotElapsed
            );

            let info =
                Validators::<T>::get(slash_record.validator).ok_or(Error::<T>::ValidatorNotFound)?;

            let _ = T::Currency::slash_reserved(&info.controller, slash_record.amount);

            let new_stake = ValidatorStake::<T>::get(slash_record.validator)
                .saturating_sub(slash_record.amount);
            ValidatorStake::<T>::insert(slash_record.validator, new_stake);
            TotalStake::<T>::mutate(|total| {
                *total = total.saturating_sub(slash_record.amount);
            });

            let mut info_mut = info;
            info_mut.stake = new_stake;
            Validators::<T>::insert(slash_record.validator, info_mut);

            slash_record.applied = true;
            PendingSlashes::<T>::insert(slash_id, slash_record.clone());

            Self::deposit_event(Event::SlashApplied {
                validator: slash_record.validator,
                amount: slash_record.amount,
            });

            Ok(())
        }

        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::report_evidence())]
        pub fn report_evidence(
            origin: OriginFor<T>,
            validator: ValidatorId,
            violation: ViolationType,
        ) -> DispatchResult {
            let reporter = ensure_signed(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let info = Validators::<T>::get(validator).ok_or(Error::<T>::ValidatorNotFound)?;
            let stake = ValidatorStake::<T>::get(validator);

            let slash_pct = Self::get_slash_percentage(&violation);
            let slash_amount = slash_pct.mul_floor(stake);

            let reward = Self::calculate_evidence_reward(slash_amount);

            let slash_id = SlashCount::<T>::get();
            SlashCount::<T>::put(slash_id.saturating_add(1));

            let slash_record = SlashRecord {
                validator,
                amount: slash_amount,
                violation: violation.clone(),
                block: block_number,
                applied: false,
            };

            PendingSlashes::<T>::insert(slash_id, slash_record);

            Self::deposit_event(Event::ValidatorSlashed {
                validator,
                amount: slash_amount,
                violation,
            });

            if reward > BalanceOf::<T>::zero() {
                T::Currency::unreserve(&info.controller, reward);
                let _ = T::Currency::transfer(
                    &info.controller,
                    &reporter,
                    reward,
                    frame_support::traits::ExistenceRequirement::AllowDeath,
                );

                Self::deposit_event(Event::EvidenceRewardPaid {
                    reporter,
                    amount: reward,
                });
            }

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn account_to_validator(account: &T::AccountId) -> ValidatorId {
            use sp_runtime::traits::Hash;
            let hash = T::Hashing::hash_of(account);
            ValidatorId::from(sp_core::H256(hash.as_ref().try_into().unwrap_or([0u8; 32])))
        }

        fn ensure_stake_ratio_valid(stake: BalanceOf<T>) -> DispatchResult {
            let total_stake = TotalStake::<T>::get();

            if total_stake.is_zero() {
                return Ok(());
            }

            let new_total = total_stake.saturating_add(stake);
            let max_allowed = MAX_STAKE_RATIO.mul_floor(new_total);
            ensure!(stake <= max_allowed, Error::<T>::StakeTooHigh);

            Ok(())
        }

        fn get_slash_percentage(violation: &ViolationType) -> Perbill {
            match violation {
                ViolationType::Minor => SLASH_MINOR,
                ViolationType::Moderate => SLASH_MODERATE,
                ViolationType::Severe => SLASH_SEVERE,
                ViolationType::Critical => SLASH_CRITICAL,
            }
        }

        fn calculate_evidence_reward(slash_amount: BalanceOf<T>) -> BalanceOf<T> {
            let reward_pct = Perbill::from_percent(10);
            let reward = reward_pct.mul_floor(slash_amount);
            let max_reward = BalanceOf::<T>::from(EVIDENCE_REWARD_MAX as u32);
            if reward > max_reward {
                max_reward
            } else {
                reward
            }
        }

        pub fn get_validator(validator: ValidatorId) -> Option<ValidatorInfo<T>> {
            Validators::<T>::get(validator)
        }

        pub fn is_validator_active(validator: ValidatorId) -> bool {
            Validators::<T>::get(validator)
                .map(|info| info.status == ValidatorStatus::Active)
                .unwrap_or(false)
        }

        pub fn get_active_validators() -> Vec<ValidatorId> {
            Validators::<T>::iter()
                .filter_map(|(id, info)| {
                    if info.status == ValidatorStatus::Active {
                        Some(id)
                    } else {
                        None
                    }
                })
                .collect()
        }

        pub fn get_stake_ratio(validator: ValidatorId) -> Option<(BalanceOf<T>, BalanceOf<T>)> {
            let stake = ValidatorStake::<T>::get(validator);
            let total = TotalStake::<T>::get();
            if total.is_zero() {
                None
            } else {
                Some((stake, total))
            }
        }
    }
}
