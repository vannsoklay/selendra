// This file is part of Selendra.

// Copyright (C) 2020-2022 Selendra.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

#![cfg(any(test, feature = "bench"))]

use crate::{AllPrecompiles, Ratio, RuntimeBlockWeights, Weight};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	ord_parameter_types, parameter_types,
	traits::{
		ConstU128, ConstU32, ConstU64, EqualPrivilegeOnly, Everything, InstanceFilter, Nothing,
		OnFinalize, OnInitialize, SortedMembers,
	},
	weights::IdentityFee,
	PalletId, RuntimeDebug,
};
use frame_system::{offchain::SendTransactionTypes, EnsureRoot, EnsureSignedBy};
use module_evm::{EvmChainId, EvmTask};
use module_evm_accounts::EvmAddressMapping;
use module_support::{
	AddressMapping as AddressMappingT, DEXIncentives, DispatchableTask, EmergencyShutdown,
	ExchangeRate, ExchangeRateProvider, PoolId, PriceProvider, Rate,
};
use orml_traits::{parameter_type_with_key, MultiReservableCurrency};
pub use primitives::{
	define_combined_task,
	evm::{convert_decimals_to_evm, EvmAddress},
	task::TaskResult,
	Address, Amount, AuctionId, BlockNumber, CurrencyId, DexShare, EraIndex, Header, Lease, Moment,
	Nonce, ReserveIdentifier, Signature, TokenSymbol, TradingPair,
};
use scale_info::TypeInfo;
use sp_core::{H160, H256};
use sp_runtime::{
	traits::{
		AccountIdConversion, BlakeTwo256, BlockNumberProvider, Convert, IdentityLookup,
		One as OneT, Zero,
	},
	AccountId32, DispatchResult, FixedPointNumber, FixedU128, Perbill, Percent, Permill,
};
use sp_std::prelude::*;

pub type AccountId = AccountId32;
type Key = CurrencyId;
pub type Price = FixedU128;
type Balance = u128;

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = ();
	type Origin = Origin;
	type Call = Call;
	type Index = Nonce;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = ConstU32<250>;
	type DbWeight = frame_support::weights::constants::RocksDbWeight;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub const ExistenceRequirement: u128 = 1;
	pub const MinimumCount: u32 = 1;
	pub const ExpiresIn: u32 = 600;
	pub const RootOperatorAccountId: AccountId = ALICE;
	pub OracleMembers: Vec<AccountId> = vec![ALICE, BOB, EVA];
}

pub struct Members;

impl SortedMembers<AccountId> for Members {
	fn sorted_members() -> Vec<AccountId> {
		OracleMembers::get()
	}
}

impl orml_oracle::Config for Test {
	type Event = Event;
	type OnNewData = ();
	type CombineData = orml_oracle::DefaultCombineData<Self, MinimumCount, ExpiresIn>;
	type Time = Timestamp;
	type OracleKey = Key;
	type OracleValue = Price;
	type RootOperatorAccountId = RootOperatorAccountId;
	type Members = Members;
	type WeightInfo = ();
	type MaxHasDispatchedSize = ConstU32<40>;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ();
	type WeightInfo = ();
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type DustRemovalWhitelist = Nothing;
	type OnNewTokenAccount = ();
	type OnKilledTokenAccount = ();
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistenceRequirement;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = ReserveIdentifier;
}

pub const SEL: CurrencyId = CurrencyId::Token(TokenSymbol::SEL);
pub const RENBTC: CurrencyId = CurrencyId::Token(TokenSymbol::RENBTC);
pub const SUSD: CurrencyId = CurrencyId::Token(TokenSymbol::SUSD);
pub const DOT: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
pub const LP_SEL_SUSD: CurrencyId =
	CurrencyId::DexShare(DexShare::Token(TokenSymbol::SEL), DexShare::Token(TokenSymbol::SUSD));

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = SEL;
	pub Erc20HoldingAccount: H160 = H160::from_low_u64_be(1);
}

impl module_currencies::Config for Test {
	type Event = Event;
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type Erc20HoldingAccount = Erc20HoldingAccount;
	type WeightInfo = ();
	type AddressMapping = EvmAddressMapping<Test>;
	type EVMBridge = module_evm_bridge::EVMBridge<Test>;
	type GasToWeight = ();
	type SweepOrigin = EnsureSignedBy<CouncilAccount, AccountId>;
	type OnDust = ();
}

impl module_evm_bridge::Config for Test {
	type EVM = EVMModule;
}

impl module_asset_registry::Config for Test {
	type Event = Event;
	type Currency = Balances;
	type StakingCurrencyId = GetStakingCurrencyId;
	type EVMBridge = module_evm_bridge::EVMBridge<Test>;
	type RegisterOrigin = EnsureSignedBy<CouncilAccount, AccountId>;
	type WeightInfo = ();
}

define_combined_task! {
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	pub enum ScheduledTasks {
		EvmTask(EvmTask<Test>),
	}
}

pub struct MockBlockNumberProvider;

impl BlockNumberProvider for MockBlockNumberProvider {
	type BlockNumber = u32;

	fn current_block_number() -> Self::BlockNumber {
		Zero::zero()
	}
}

impl module_idle_scheduler::Config for Test {
	type Event = Event;
	type WeightInfo = ();
	type Task = ScheduledTasks;
	type MinimumWeightRemainInBlock = ConstU64<0>;
	type RelayChainBlockNumberProvider = MockBlockNumberProvider;
	type DisableBlockThreshold = ConstU32<6>;
}

parameter_types! {
	pub const NftPalletId: PalletId = PalletId(*b"sel/aNFT");
}
impl module_nft::Config for Test {
	type Event = Event;
	type Currency = Balances;
	type CreateClassDeposit = ConstU128<200>;
	type CreateTokenDeposit = ConstU128<100>;
	type DataDepositPerByte = ConstU128<10>;
	type PalletId = NftPalletId;
	type MaxAttributesBytes = ConstU32<2048>;
	type WeightInfo = ();
}

impl orml_nft::Config for Test {
	type ClassId = u32;
	type TokenId = u64;
	type ClassData = module_nft::ClassData<Balance>;
	type TokenData = module_nft::TokenData<Balance>;
	type MaxClassMetadata = ConstU32<1024>;
	type MaxTokenMetadata = ConstU32<1024>;
}

parameter_types! {
	pub const GetStableCurrencyId: CurrencyId = SUSD;
	pub MaxSwapSlippageCompareToOracle: Ratio = Ratio::one();
	pub const TreasuryPalletId: PalletId = PalletId(*b"sel/trsy");
	pub const TransactionPaymentPalletId: PalletId = PalletId(*b"sel/fees");
	pub CardamomTreasuryAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
	pub const CustomFeeSurplus: Percent = Percent::from_percent(50);
	pub const AlternativeFeeSurplus: Percent = Percent::from_percent(25);
	pub DefaultFeeTokens: Vec<CurrencyId> = vec![SUSD];
}

impl module_transaction_payment::Config for Test {
	type Event = Event;
	type Call = Call;
	type NativeCurrencyId = GetNativeCurrencyId;
	type Currency = Balances;
	type MultiCurrency = Currencies;
	type OnTransactionPayment = ();
	type OperationalFeeMultiplier = ConstU64<5>;
	type TipPerWeightStep = ConstU128<1>;
	type MaxTipsOfPriority = ConstU128<1000>;
	type AlternativeFeeSwapDeposit = ExistenceRequirement;
	type WeightToFee = IdentityFee<Balance>;
	type TransactionByteFee = ConstU128<10>;
	type FeeMultiplierUpdate = ();
	type DEX = DexModule;
	type MaxSwapSlippageCompareToOracle = MaxSwapSlippageCompareToOracle;
	type TradingPathLimit = TradingPathLimit;
	type PriceSource = module_prices::RealTimePriceProvider<Test>;
	type WeightInfo = ();
	type PalletId = TransactionPaymentPalletId;
	type TreasuryAccount = CardamomTreasuryAccount;
	type UpdateOrigin = EnsureSignedBy<ListingOrigin, AccountId>;
	type CustomFeeSurplus = CustomFeeSurplus;
	type AlternativeFeeSurplus = AlternativeFeeSurplus;
	type DefaultFeeTokens = DefaultFeeTokens;
}

#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	TypeInfo,
)]
pub enum ProxyType {
	Any,
	JustTransfer,
	JustUtility,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<Call> for ProxyType {
	fn filter(&self, c: &Call) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::JustTransfer =>
				matches!(c, Call::Balances(pallet_balances::Call::transfer { .. })),
			ProxyType::JustUtility => matches!(c, Call::Utility { .. }),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		self == &ProxyType::Any || self == o
	}
}

impl pallet_proxy::Config for Test {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ConstU128<1>;
	type ProxyDepositFactor = ConstU128<1>;
	type MaxProxies = ConstU32<4>;
	type WeightInfo = ();
	type MaxPending = ConstU32<2>;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = ConstU128<1>;
	type AnnouncementDepositFactor = ConstU128<1>;
}

impl pallet_utility::Config for Test {
	type Event = Event;
	type Call = Call;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = ();
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(10) * RuntimeBlockWeights::get().max_block;
}

impl pallet_scheduler::Config for Test {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type MaxScheduledPerBlock = ConstU32<50>;
	type WeightInfo = ();
	type PreimageProvider = ();
	type NoPreimagePostponement = ();
}

pub struct MockDEXIncentives;
impl DEXIncentives<AccountId, CurrencyId, Balance> for MockDEXIncentives {
	fn do_deposit_dex_share(
		who: &AccountId,
		lp_currency_id: CurrencyId,
		amount: Balance,
	) -> DispatchResult {
		Tokens::reserve(lp_currency_id, who, amount)
	}

	fn do_withdraw_dex_share(
		who: &AccountId,
		lp_currency_id: CurrencyId,
		amount: Balance,
	) -> DispatchResult {
		let _ = Tokens::unreserve(lp_currency_id, who, amount);
		Ok(())
	}
}

ord_parameter_types! {
	pub const ListingOrigin: AccountId = ALICE;
}

parameter_types! {
	pub const GetExchangeFee: (u32, u32) = (1, 100);
	pub const TradingPathLimit: u32 = 4;
	pub const DEXPalletId: PalletId = PalletId(*b"sel/dexm");
}

impl module_dex::Config for Test {
	type Event = Event;
	type Currency = Tokens;
	type GetExchangeFee = GetExchangeFee;
	type TradingPathLimit = TradingPathLimit;
	type PalletId = DEXPalletId;
	type Erc20InfoMapping = EvmErc20InfoMapping;
	type WeightInfo = ();
	type DEXIncentives = MockDEXIncentives;
	type ListingOrigin = EnsureSignedBy<ListingOrigin, AccountId>;
	type ExtendedProvisioningBlocks = ConstU32<0>;
	type OnLiquidityPoolUpdated = ();
}

pub struct MockPriceSource;
impl PriceProvider<CurrencyId> for MockPriceSource {
	fn get_relative_price(_base: CurrencyId, _quote: CurrencyId) -> Option<Price> {
		Some(Price::one())
	}

	fn get_price(_currency_id: CurrencyId) -> Option<Price> {
		Some(Price::one())
	}
}

pub struct MockEmergencyShutdown;
impl EmergencyShutdown for MockEmergencyShutdown {
	fn is_shutdown() -> bool {
		false
	}
}

parameter_types! {
	pub const SelTreasuryPalletId: PalletId = PalletId(*b"sel/cdpt");
	pub SelTreasuryAccount: AccountId = PalletId(*b"sel/hztr").into_account_truncating();
	pub AlternativeSwapPathJointList: Vec<Vec<CurrencyId>> = vec![
		vec![SUSD],
	];
}

impl module_treasury::Config for Test {
	type Currency = Currencies;
	type GetStableCurrencyId = GetStableCurrencyId;
	type PalletId = SelTreasuryPalletId;
}

pub type AdaptedBasicCurrency =
	module_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

pub type EvmErc20InfoMapping = module_asset_registry::EvmErc20InfoMapping<Test>;

parameter_types! {
	pub NetworkContractSource: H160 = alice_evm_addr();
	pub PrecompilesValue: AllPrecompiles<Test> = AllPrecompiles::<_>::mandala();
}

ord_parameter_types! {
	pub const CouncilAccount: AccountId32 = AccountId32::from([1u8; 32]);
	pub const TreasuryAccount: AccountId32 = AccountId32::from([2u8; 32]);
	pub const NetworkContractAccount: AccountId32 = AccountId32::from([0u8; 32]);
	pub const StorageDepositPerByte: u128 = convert_decimals_to_evm(10);
}

pub struct GasToWeight;
impl Convert<u64, Weight> for GasToWeight {
	fn convert(a: u64) -> u64 {
		a as Weight
	}
}

impl module_evm::Config for Test {
	type AddressMapping = EvmAddressMapping<Test>;
	type Currency = Balances;
	type TransferAll = Currencies;
	type NewContractExtraBytes = ConstU32<100>;
	type StorageDepositPerByte = StorageDepositPerByte;
	type TxFeePerGas = ConstU128<10>;
	type Event = Event;
	type PrecompilesType = AllPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type GasToWeight = GasToWeight;
	type ChargeTransactionPayment = module_transaction_payment::ChargeTransactionPayment<Test>;
	type NetworkContractOrigin = EnsureSignedBy<NetworkContractAccount, AccountId>;
	type NetworkContractSource = NetworkContractSource;
	type DeveloperDeposit = ConstU128<1000>;
	type PublicationFee = ConstU128<200>;
	type TreasuryAccount = TreasuryAccount;
	type FreePublicationOrigin = EnsureSignedBy<CouncilAccount, AccountId>;
	type Runner = module_evm::runner::stack::Runner<Self>;
	type FindAuthor = ();
	type Task = ScheduledTasks;
	type IdleScheduler = IdleScheduler;
	type WeightInfo = ();
}

impl module_evm_accounts::Config for Test {
	type Event = Event;
	type Currency = Balances;
	type AddressMapping = EvmAddressMapping<Test>;
	type ChainId = EvmChainId<Test>;
	type TransferAll = ();
	type WeightInfo = ();
}

pub struct MockLiquidStakingExchangeProvider;
impl ExchangeRateProvider for MockLiquidStakingExchangeProvider {
	fn get_exchange_rate() -> ExchangeRate {
		ExchangeRate::saturating_from_rational(1, 2)
	}
}

impl BlockNumberProvider for MockRelayBlockNumberProvider {
	type BlockNumber = BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		Self::get()
	}
}

parameter_type_with_key! {
	pub LiquidCrowdloanLeaseBlockNumber: |_lease: Lease| -> Option<BlockNumber> {
		None
	};
}

parameter_type_with_key! {
	pub PricingPegged: |_currency_id: CurrencyId| -> Option<CurrencyId> {
		None
	};
}

parameter_types! {
	pub StableCurrencyFixedPrice: Price = Price::saturating_from_rational(1, 1);
	pub const GetStakingCurrencyId: CurrencyId = DOT;
	pub MockRelayBlockNumberProvider: BlockNumber = 0;
	pub RewardRatePerRelaychainBlock: Rate = Rate::zero();
}

ord_parameter_types! {
	pub const One: AccountId = AccountId::new([1u8; 32]);
}

impl module_prices::Config for Test {
	type Event = Event;
	type Source = Oracle;
	type GetStableCurrencyId = GetStableCurrencyId;
	type StableCurrencyFixedPrice = StableCurrencyFixedPrice;
	type LockOrigin = EnsureSignedBy<One, AccountId>;
	type DEX = DexModule;
	type Currency = Currencies;
	type Erc20InfoMapping = EvmErc20InfoMapping;
	type WeightInfo = ();
}

impl orml_rewards::Config for Test {
	type Share = Balance;
	type Balance = Balance;
	type PoolId = PoolId;
	type CurrencyId = CurrencyId;
	type Handler = Incentives;
}

parameter_types! {
	pub const IncentivesPalletId: PalletId = PalletId(*b"sel/inct");
}

ord_parameter_types! {
	pub const EarnShareBooster: Permill = Permill::from_percent(50);
	pub const RewardsSource: AccountId = REWARDS_SOURCE;
}

impl module_incentives::Config for Test {
	type Event = Event;
	type RewardsSource = RewardsSource;
	type AccumulatePeriod = ConstU32<10>;
	type StableCurrencyId = GetStableCurrencyId;
	type NativeCurrencyId = GetNativeCurrencyId;
	type EarnShareBooster = EarnShareBooster;
	type UpdateOrigin = EnsureSignedBy<One, AccountId>;
	type SelTreasury = SelTreasury;
	type Currency = Tokens;
	type DEX = DexModule;
	type EmergencyShutdown = MockEmergencyShutdown;
	type PalletId = IncentivesPalletId;
	type WeightInfo = ();
}

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const EVA: AccountId = AccountId::new([5u8; 32]);
pub const REWARDS_SOURCE: AccountId = AccountId::new([3u8; 32]);
pub const HOMA_TREASURY: AccountId = AccountId::new([255u8; 32]);

pub fn alice() -> AccountId {
	<Test as module_evm::Config>::AddressMapping::get_account_id(&alice_evm_addr())
}

pub fn alice_evm_addr() -> EvmAddress {
	EvmAddress::from(hex_literal::hex!("1000000000000000000000000000000000000001"))
}

pub fn bob() -> AccountId {
	<Test as module_evm::Config>::AddressMapping::get_account_id(&bob_evm_addr())
}

pub fn bob_evm_addr() -> EvmAddress {
	EvmAddress::from(hex_literal::hex!("1000000000000000000000000000000000000002"))
}

pub fn sel_evm_address() -> EvmAddress {
	EvmAddress::try_from(SEL).unwrap()
}

pub fn ausd_evm_address() -> EvmAddress {
	EvmAddress::try_from(SUSD).unwrap()
}

pub fn lp_sel_ausd_evm_address() -> EvmAddress {
	EvmAddress::try_from(LP_SEL_SUSD).unwrap()
}

pub fn erc20_address_not_exists() -> EvmAddress {
	EvmAddress::from(hex_literal::hex!("0000000000000000000000000000000200000001"))
}

pub const INITIAL_BALANCE: Balance = 1_000_000_000_000;

pub type SignedExtra = (frame_system::CheckWeight<Test>,);
pub type UncheckedExtrinsic =
	sp_runtime::generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Oracle: orml_oracle,
		Timestamp: pallet_timestamp,
		Tokens: orml_tokens exclude_parts { Call },
		Balances: pallet_balances,
		Currencies: module_currencies,
		SelTreasury: module_treasury,
		EVMBridge: module_evm_bridge exclude_parts { Call },
		AssetRegistry: module_asset_registry,
		NFTModule: module_nft,
		TransactionPayment: module_transaction_payment,
		Prices: module_prices,
		Proxy: pallet_proxy,
		Utility: pallet_utility,
		Scheduler: pallet_scheduler,
		DexModule: module_dex,
		EVMModule: module_evm,
		EvmAccounts: module_evm_accounts,
		IdleScheduler: module_idle_scheduler,
		Incentives: module_incentives,
		Rewards: orml_rewards,
	}
);

impl<LocalCall> SendTransactionTypes<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = UncheckedExtrinsic;
}

#[cfg(test)]
// This function basically just builds a genesis storage key/value store
// according to our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
	use frame_support::{assert_ok, traits::GenesisBuild};
	use sp_std::collections::btree_map::BTreeMap;

	let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let mut accounts = BTreeMap::new();
	let mut evm_genesis_accounts = crate::evm_genesis(vec![]);
	accounts.append(&mut evm_genesis_accounts);

	accounts.insert(
		alice_evm_addr(),
		module_evm::GenesisAccount { nonce: 1, balance: INITIAL_BALANCE, ..Default::default() },
	);
	accounts.insert(
		bob_evm_addr(),
		module_evm::GenesisAccount { nonce: 1, balance: INITIAL_BALANCE, ..Default::default() },
	);

	pallet_balances::GenesisConfig::<Test>::default()
		.assimilate_storage(&mut storage)
		.unwrap();
	module_evm::GenesisConfig::<Test> { chain_id: 595, accounts }
		.assimilate_storage(&mut storage)
		.unwrap();
	module_asset_registry::GenesisConfig::<Test> {
		assets: vec![(SEL, ExistenceRequirement::get()), (RENBTC, 0)],
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(storage);
	ext.execute_with(|| {
		System::set_block_number(1);
		Timestamp::set_timestamp(1);

		assert_ok!(Currencies::update_balance(Origin::root(), ALICE, RENBTC, 1_000_000_000_000));
		assert_ok!(Currencies::update_balance(Origin::root(), ALICE, SUSD, 1_000_000_000));

		assert_ok!(Currencies::update_balance(
			Origin::root(),
			EvmAddressMapping::<Test>::get_account_id(&alice_evm_addr()),
			RENBTC,
			1_000_000_000
		));

		assert_ok!(Currencies::update_balance(
			Origin::root(),
			EvmAddressMapping::<Test>::get_account_id(&alice_evm_addr()),
			SUSD,
			1_000_000_000
		));
	});
	ext
}

pub fn run_to_block(n: u32) {
	while System::block_number() < n {
		Scheduler::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		Scheduler::on_initialize(System::block_number());
	}
}
