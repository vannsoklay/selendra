// This file is part of Selendra.

// Copyright (C) 2021-2022 Selendra.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::all)]

use primitives::evm::{
	AccessListItem, BlockLimits, CallInfo, CreateInfo, EstimateResourcesRequest,
};
use sp_core::H160;
use sp_runtime::{
	codec::Codec,
	traits::{MaybeDisplay, MaybeFromStr},
};
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
	#[api_version(2)]
	pub trait EVMRuntimeRPCApi<Balance> where
		Balance: Codec + MaybeDisplay + MaybeFromStr,
	{
		fn call(
			from: H160,
			to: H160,
			data: Vec<u8>,
			value: Balance,
			gas_limit: u64,
			storage_limit: u32,
			access_list: Option<Vec<AccessListItem>>,
			estimate: bool,
		) -> Result<CallInfo, sp_runtime::DispatchError>;

		fn create(
			from: H160,
			data: Vec<u8>,
			value: Balance,
			gas_limit: u64,
			storage_limit: u32,
			access_list: Option<Vec<AccessListItem>>,
			estimate: bool,
		) -> Result<CreateInfo, sp_runtime::DispatchError>;

		fn get_estimate_resources_request(data: Vec<u8>) -> Result<EstimateResourcesRequest, sp_runtime::DispatchError>;

		fn block_limits() -> BlockLimits;
	}
}
