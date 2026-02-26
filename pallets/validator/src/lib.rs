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
    use alloc::vec::Vec;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, Get, ReservableCurrency, StorageVersion},
    };
    use frame_system::pallet_prelude::*;
    use seveny_primitives::{
        constants::{
            EVIDENCE_REWARD_MAX, MAX_STAKE_RATIO, SLASH_CRITICAL, SLASH_MINOR, SLASH_MODERATE,
            SLASH_SEVERE,
        },
        types::{ValidatorId, ViolationType},
    };
    use sp_arithmetic::Perbill;
    use sp_runtime::{traits::Zero, Saturating};

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
        type MinValidators: Get<u32>;

        #[pallet::constant]
        type BondingDuration: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type SlashDeferDuration: Get<BlockNumberFor<Self>>;
    }

    #[derive(
        Clone,
        PartialEq,
        Eq,
        Encode,
        Decode,
        parity_scale_codec::DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    pub enum ValidatorStatus {
        Bonding,
        Active,
        Unbonding,
        Slashed,
    }

    #[derive(
        Clone,
        PartialEq,
        Eq,
        Encode,
        Decode,
        parity_scale_codec::DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct ValidatorInfo<T: Config> {
        pub id: ValidatorId,
        pub controller: T::AccountId,
        pub stake: BalanceOf<T>,
        pub status: ValidatorStatus,
        pub registered_at: BlockNumberFor<T>,
        pub unbonding_at: Option<BlockNumberFor<T>>,
    }

    #[derive(
        Clone,
        PartialEq,
        Eq,
        Encode,
        Decode,
        parity_scale_codec::DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct SlashRecord<T: Config> {
        pub validator: ValidatorId,
        pub amount: BalanceOf<T>,
        pub violation: ViolationType,
        pub block: BlockNumberFor<T>,
        pub applied: bool,
        /// Reporter who submitted evidence (None for root-initiated slashes)
        pub reporter: Option<T::AccountId>,
    }

    #[pallet::storage]
    #[pallet::getter(fn validators)]
    pub type Validators<T: Config> =
        StorageMap<_, Blake2_128Concat, ValidatorId, ValidatorInfo<T>, OptionQuery>;

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
    pub type SlashDedup<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ValidatorId,
        Blake2_128Concat,
        ViolationType,
        BlockNumberFor<T>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn slash_count)]
    pub type SlashCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Tracks evidence submissions: (validator, reporter) -> block number.
    /// Prevents the same reporter from filing duplicate evidence against
    /// the same validator.
    #[pallet::storage]
    pub type EvidenceSubmissions<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ValidatorId,
        Blake2_128Concat,
        T::AccountId,
        BlockNumberFor<T>,
    >;

    /// Rate limit tracker: reporter -> (window_start_block, count_in_window).
    /// Limits evidence reports per reporter to prevent spam.
    #[pallet::storage]
    pub type EvidenceReportCount<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, (BlockNumberFor<T>, u32)>;

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
        /// Duplicate evidence submission for this validator by this reporter
        DuplicateEvidence,
        /// Evidence report rate limit exceeded
        EvidenceRateLimitExceeded,
        DuplicateSlash,
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
            for (controller, stake) in &self.initial_validators {
                let validator_id =
                    seveny_primitives::crypto::derive_validator_id(&controller.encode());

                let info = ValidatorInfo {
                    id: validator_id,
                    controller: controller.clone(),
                    stake: *stake,
                    status: ValidatorStatus::Active,
                    registered_at: BlockNumberFor::<T>::zero(),
                    unbonding_at: None,
                };

                Validators::<T>::insert(validator_id, info);
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

            Self::ensure_stake_ratio_valid(stake, stake)?;

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
            let mut info =
                Validators::<T>::get(validator_id).ok_or(Error::<T>::ValidatorNotFound)?;

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
            let mut info =
                Validators::<T>::get(validator_id).ok_or(Error::<T>::ValidatorNotFound)?;

            ensure!(
                info.status == ValidatorStatus::Active,
                Error::<T>::NotActive
            );

            let active_count = ActiveValidatorCount::<T>::get();
            ensure!(
                active_count > T::MinValidators::get(),
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

            let stake = info.stake;
            T::Currency::unreserve(&who, stake);

            Validators::<T>::remove(validator_id);
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
            let mut info =
                Validators::<T>::get(validator_id).ok_or(Error::<T>::ValidatorNotFound)?;

            let new_stake = info.stake.saturating_add(additional);
            Self::ensure_stake_ratio_valid(new_stake, additional)?;

            T::Currency::reserve(&who, additional)?;

            info.stake = new_stake;
            Validators::<T>::insert(validator_id, info);
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

            // Prevent duplicate slash for same validator + violation type
            ensure!(
                !SlashDedup::<T>::contains_key(validator, violation),
                Error::<T>::DuplicateSlash
            );

            let info = Validators::<T>::get(validator).ok_or(Error::<T>::ValidatorNotFound)?;

            let slash_pct = Self::get_slash_percentage(&violation);
            let slash_amount = slash_pct.mul_floor(info.stake);

            let slash_id = SlashCount::<T>::get();
            SlashCount::<T>::put(slash_id.saturating_add(1));

            let defer_until = block_number.saturating_add(T::SlashDeferDuration::get());

            let slash_record = SlashRecord {
                validator,
                amount: slash_amount,
                violation,
                block: block_number,
                applied: false,
                reporter: None,
            };

            PendingSlashes::<T>::insert(slash_id, slash_record);
            SlashDedup::<T>::insert(validator, violation, block_number);

            Self::deposit_event(Event::SlashDeferred {
                validator,
                amount: slash_amount,
                defer_until,
            });

            if violation == ViolationType::Critical && info.status != ValidatorStatus::Slashed {
                // H04: only decrement ActiveValidatorCount for Active validators
                let was_active = info.status == ValidatorStatus::Active;
                let mut info_mut = info;
                info_mut.status = ValidatorStatus::Slashed;
                Validators::<T>::insert(validator, info_mut);

                if was_active {
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

            let info = Validators::<T>::get(slash_record.validator)
                .ok_or(Error::<T>::ValidatorNotFound)?;

            let _ = T::Currency::slash_reserved(&info.controller, slash_record.amount);

            let new_stake = info.stake.saturating_sub(slash_record.amount);
            TotalStake::<T>::mutate(|total| {
                *total = total.saturating_sub(slash_record.amount);
            });

            let mut info_mut = info;
            info_mut.stake = new_stake;
            Validators::<T>::insert(slash_record.validator, info_mut);

            slash_record.applied = true;
            PendingSlashes::<T>::insert(slash_id, slash_record.clone());

            // Clear dedup entry so a new slash can be created for this violation type
            SlashDedup::<T>::remove(slash_record.validator, slash_record.violation);

            Self::deposit_event(Event::SlashApplied {
                validator: slash_record.validator,
                amount: slash_record.amount,
            });

            // C04: pay evidence reward from slash imbalance, not from
            // the validator's free balance (which would double-punish).
            if let Some(ref reporter) = slash_record.reporter {
                let reward = Self::calculate_evidence_reward(slash_record.amount);
                if reward > BalanceOf::<T>::zero() {
                    let _ = T::Currency::deposit_creating(reporter, reward);

                    Self::deposit_event(Event::EvidenceRewardPaid {
                        reporter: reporter.clone(),
                        amount: reward,
                    });
                }
            }

            Ok(())
        }

        /// Report evidence of a validator violation.
        ///
        /// Security hardening:
        /// - Duplicate detection: same reporter cannot report same validator twice
        /// - Rate limiting: max 3 reports per reporter per 100-block window
        /// - Deferred rewards: reward paid when slash is applied, not at report time
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::report_evidence())]
        pub fn report_evidence(
            origin: OriginFor<T>,
            validator: ValidatorId,
            violation: ViolationType,
        ) -> DispatchResult {
            let reporter = ensure_signed(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            // Duplicate evidence check
            ensure!(
                !EvidenceSubmissions::<T>::contains_key(validator, &reporter),
                Error::<T>::DuplicateEvidence
            );

            // C03: prevent stacking slashes for same validator + violation type
            ensure!(
                !SlashDedup::<T>::contains_key(validator, violation),
                Error::<T>::DuplicateSlash
            );

            // Rate limiting: max 3 reports per 100-block window
            const MAX_REPORTS_PER_WINDOW: u32 = 3;
            const RATE_LIMIT_WINDOW: u64 = 100;
            let window_blocks = BlockNumberFor::<T>::from(RATE_LIMIT_WINDOW as u32);
            if let Some((window_start, count)) = EvidenceReportCount::<T>::get(&reporter) {
                if block_number < window_start.saturating_add(window_blocks) {
                    ensure!(
                        count < MAX_REPORTS_PER_WINDOW,
                        Error::<T>::EvidenceRateLimitExceeded
                    );
                    EvidenceReportCount::<T>::insert(
                        &reporter,
                        (window_start, count.saturating_add(1)),
                    );
                } else {
                    // New window
                    EvidenceReportCount::<T>::insert(&reporter, (block_number, 1u32));
                }
            } else {
                EvidenceReportCount::<T>::insert(&reporter, (block_number, 1u32));
            }

            let info = Validators::<T>::get(validator).ok_or(Error::<T>::ValidatorNotFound)?;

            let slash_pct = Self::get_slash_percentage(&violation);
            let slash_amount = slash_pct.mul_floor(info.stake);

            let slash_id = SlashCount::<T>::get();
            SlashCount::<T>::put(slash_id.saturating_add(1));

            let slash_record = SlashRecord {
                validator,
                amount: slash_amount,
                violation,
                block: block_number,
                applied: false,
                reporter: Some(reporter.clone()),
            };

            PendingSlashes::<T>::insert(slash_id, slash_record);
            EvidenceSubmissions::<T>::insert(validator, &reporter, block_number);
            SlashDedup::<T>::insert(validator, violation, block_number);

            Self::deposit_event(Event::ValidatorSlashed {
                validator,
                amount: slash_amount,
                violation,
            });

            // Reward is deferred to apply_slash — not paid immediately
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn account_to_validator(account: &T::AccountId) -> ValidatorId {
            seveny_primitives::crypto::derive_validator_id(&account.encode())
        }

        /// Check that a validator's stake does not exceed MAX_STAKE_RATIO of
        /// the total after the operation.
        ///
        /// `validator_stake`: the validator's full stake after the operation.
        /// `additional_to_total`: amount being added to TotalStake (avoids
        ///   double-counting when increase_stake passes full new_stake).
        fn ensure_stake_ratio_valid(
            validator_stake: BalanceOf<T>,
            additional_to_total: BalanceOf<T>,
        ) -> DispatchResult {
            let total_stake = TotalStake::<T>::get();

            // During bootstrap (no existing stake), allow registration
            if total_stake.is_zero() {
                return Ok(());
            }

            let total_after = total_stake.saturating_add(additional_to_total);
            let max_allowed = MAX_STAKE_RATIO.mul_floor(total_after);
            ensure!(validator_stake <= max_allowed, Error::<T>::StakeTooHigh);

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

        pub fn validator_stake(validator: ValidatorId) -> BalanceOf<T> {
            Validators::<T>::get(validator)
                .map(|info| info.stake)
                .unwrap_or_default()
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
            let info = Validators::<T>::get(validator)?;
            let total = TotalStake::<T>::get();
            if total.is_zero() {
                None
            } else {
                Some((info.stake, total))
            }
        }
    }

    impl<T: Config> seveny_primitives::traits::ValidatorProvider for Pallet<T> {
        fn is_validator_active(validator_id: ValidatorId) -> bool {
            Self::is_validator_active(validator_id)
        }
    }
}
