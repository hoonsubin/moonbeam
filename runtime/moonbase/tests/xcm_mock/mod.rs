// Copyright 2021 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

mod parachain;
mod relay_chain;

use moonbeam_core_primitives::AccountId;
use sp_runtime::AccountId32;
use xcm::v0::{MultiAsset, MultiLocation};
use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};
pub const PARAALICE: [u8; 20] = [1u8; 20];
pub const RELAYALICE: AccountId32 = AccountId32::new([0u8; 32]);

decl_test_parachain! {
	pub struct ParaA {
		Runtime = parachain::Runtime,
		XcmpMessageHandler = parachain::MsgQueue,
		DmpMessageHandler = parachain::MsgQueue,
		new_ext = para_ext(1),
	}
}

decl_test_parachain! {
	pub struct ParaB {
		Runtime = parachain::Runtime,
		XcmpMessageHandler = parachain::MsgQueue,
		DmpMessageHandler = parachain::MsgQueue,
		new_ext = para_ext(2),
	}
}

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = relay_chain::Runtime,
		XcmConfig = relay_chain::XcmConfig,
		new_ext = relay_ext(),
	}
}

decl_test_network! {
	pub struct MockNet {
		relay_chain = Relay,
		parachains = vec![
			(1, ParaA),
			(2, ParaB),
		],
	}
}

pub const INITIAL_BALANCE: u128 = 1_000_000_000;

pub fn para_ext(para_id: u32) -> sp_io::TestExternalities {
	use parachain::{MsgQueue, Runtime, System};

	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(PARAALICE.into(), INITIAL_BALANCE)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
		MsgQueue::set_para_id(para_id.into());
	});
	ext
}

pub fn relay_ext() -> sp_io::TestExternalities {
	use relay_chain::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(RELAYALICE, INITIAL_BALANCE)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub type RelayChainPalletXcm = pallet_xcm::Pallet<relay_chain::Runtime>;
pub type ParachainPalletXcm = pallet_xcm::Pallet<parachain::Runtime>;
pub type Assets = pallet_assets::Pallet<parachain::Runtime>;
pub type XTokens = orml_xtokens::Pallet<parachain::Runtime>;
pub type RelayBalances = pallet_balances::Pallet<relay_chain::Runtime>;
pub type ParaBalances = pallet_balances::Pallet<parachain::Runtime>;

#[cfg(test)]
mod tests {
	use super::*;

	use frame_support::assert_ok;
	use pallet_crowdloan_rewards::PALLET_ID;
	use parity_scale_codec::Encode;
	use xcm::v0::{
		Junction::{self, Parachain, Parent},
		MultiAsset::*,
		MultiLocation::*,
		NetworkId, OriginKind,
		Xcm::*,
	};
	use xcm_simulator::{MultiAsset, TestExt};

	#[test]
	fn dmp() {
		MockNet::reset();

		let remark = parachain::Call::System(
			frame_system::Call::<parachain::Runtime>::remark_with_event(vec![1, 2, 3]),
		);
		Relay::execute_with(|| {
			assert_ok!(RelayChainPalletXcm::send_xcm(
				Null,
				X1(Parachain(1)),
				Transact {
					origin_type: OriginKind::SovereignAccount,
					require_weight_at_most: INITIAL_BALANCE as u64,
					call: remark.encode().into(),
				},
			));
		});

		ParaA::execute_with(|| {
			use parachain::{Event, System};
			assert!(System::events()
				.iter()
				.any(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))));
		});
	}
	#[test]
	fn ump() {
		MockNet::reset();

		let remark = relay_chain::Call::System(
			frame_system::Call::<relay_chain::Runtime>::remark_with_event(vec![1, 2, 3]),
		);
		ParaA::execute_with(|| {
			assert_ok!(ParachainPalletXcm::send_xcm(
				Null,
				X1(Parent),
				Transact {
					origin_type: OriginKind::SovereignAccount,
					require_weight_at_most: INITIAL_BALANCE as u64,
					call: remark.encode().into(),
				},
			));
		});

		Relay::execute_with(|| {
			use relay_chain::{Event, System};
			assert!(System::events()
				.iter()
				.any(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))));
		});
	}

	#[test]
	fn xcmp() {
		MockNet::reset();

		let remark = parachain::Call::System(
			frame_system::Call::<parachain::Runtime>::remark_with_event(vec![1, 2, 3]),
		);
		ParaA::execute_with(|| {
			assert_ok!(ParachainPalletXcm::send_xcm(
				Null,
				X2(Parent, Parachain(2)),
				Transact {
					origin_type: OriginKind::SovereignAccount,
					require_weight_at_most: INITIAL_BALANCE as u64,
					call: remark.encode().into(),
				},
			));
		});

		ParaB::execute_with(|| {
			use parachain::{Event, System};
			assert!(System::events()
				.iter()
				.any(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))));
		});
	}

	#[ignore]
	#[test]
	fn receive_relay_asset_from_relay() {
		MockNet::reset();

		ParaA::execute_with(|| {
			assert_ok!(Assets::force_create(
				parachain::Origin::root(),
				0u32,
				PARAALICE.into(),
				true,
				1u128
			));
		});

		Relay::execute_with(|| {
			assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
				relay_chain::Origin::signed(RELAYALICE),
				X1(Parachain(1)),
				X1(Junction::AccountKey20 {
					network: NetworkId::Any,
					key: PARAALICE
				}),
				vec![ConcreteFungible {
					id: Null,
					amount: 123
				}],
				123,
			));
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			assert_eq!(Assets::balance(0, &PARAALICE.into()), 123);
		});
	}

	#[test]
	fn send_relay_asset_to_relay() {
		MockNet::reset();

		ParaA::execute_with(|| {
			assert_ok!(Assets::force_create(
				parachain::Origin::root(),
				0u32,
				PARAALICE.into(),
				true,
				1u128
			));
		});

		Relay::execute_with(|| {
			assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
				relay_chain::Origin::signed(RELAYALICE),
				X1(Parachain(1)),
				X1(Junction::AccountKey20 {
					network: NetworkId::Any,
					key: PARAALICE
				}),
				vec![ConcreteFungible {
					id: Null,
					amount: 123
				}],
				123,
			));
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			assert_eq!(Assets::balance(0, &PARAALICE.into()), 123);
		});

		let mut balance_before_sending = 0;
		Relay::execute_with(|| {
			balance_before_sending = RelayBalances::free_balance(&RELAYALICE);
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			assert_ok!(XTokens::transfer(
				parachain::Origin::signed(PARAALICE.into()),
				0,
				100,
				X2(
					Junction::Parent,
					Junction::AccountId32 {
						network: NetworkId::Any,
						id: RELAYALICE.into()
					}
				),
				4000
			));
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			assert_eq!(Assets::balance(0, &PARAALICE.into()), 23);
		});

		Relay::execute_with(|| {
			// free execution,x	 full amount received
			assert!(RelayBalances::free_balance(&RELAYALICE) > balance_before_sending);
		});
	}

	#[test]
	fn send_relay_asset_to_para_b() {
		MockNet::reset();

		ParaA::execute_with(|| {
			assert_ok!(Assets::force_create(
				parachain::Origin::root(),
				0u32,
				PARAALICE.into(),
				true,
				1u128
			));
		});

		ParaB::execute_with(|| {
			assert_ok!(Assets::force_create(
				parachain::Origin::root(),
				0u32,
				PARAALICE.into(),
				true,
				1u128
			));
		});

		Relay::execute_with(|| {
			assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
				relay_chain::Origin::signed(RELAYALICE),
				X1(Parachain(1)),
				X1(Junction::AccountKey20 {
					network: NetworkId::Any,
					key: PARAALICE
				}),
				vec![ConcreteFungible {
					id: Null,
					amount: 123
				}],
				123,
			));
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			assert_eq!(Assets::balance(0, &PARAALICE.into()), 123);
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			assert_ok!(XTokens::transfer(
				parachain::Origin::signed(PARAALICE.into()),
				0,
				100,
				X3(
					Junction::Parent,
					Junction::Parachain(2),
					Junction::AccountKey20 {
						network: NetworkId::Any,
						key: PARAALICE.into()
					}
				),
				4000
			));
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			assert_eq!(Assets::balance(0, &PARAALICE.into()), 23);
		});

		ParaB::execute_with(|| {
			// free execution, full amount received
			assert_eq!(Assets::balance(0, &PARAALICE.into()), 100);
		});
	}

	#[test]
	fn send_para_a_asset_to_para_b() {
		MockNet::reset();

		ParaB::execute_with(|| {
			assert_ok!(Assets::force_create(
				parachain::Origin::root(),
				1u32,
				PARAALICE.into(),
				true,
				1u128
			));
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			assert_ok!(XTokens::transfer(
				parachain::Origin::signed(PARAALICE.into()),
				1,
				100,
				X3(
					Junction::Parent,
					Junction::Parachain(2),
					Junction::AccountKey20 {
						network: NetworkId::Any,
						key: PARAALICE.into()
					}
				),
				4000
			));
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			assert_eq!(
				ParaBalances::free_balance(&PARAALICE.into()),
				INITIAL_BALANCE - 100
			);
		});

		ParaB::execute_with(|| {
			// free execution, full amount received
			assert_eq!(Assets::balance(1, &PARAALICE.into()), 100);
		});
	}

	#[test]
	fn send_para_a_asset_to_para_b_and_back_to_para_a() {
		MockNet::reset();

		ParaB::execute_with(|| {
			assert_ok!(Assets::force_create(
				parachain::Origin::root(),
				1u32,
				PARAALICE.into(),
				true,
				1u128
			));
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			assert_ok!(XTokens::transfer(
				parachain::Origin::signed(PARAALICE.into()),
				1,
				100,
				X3(
					Junction::Parent,
					Junction::Parachain(2),
					Junction::AccountKey20 {
						network: NetworkId::Any,
						key: PARAALICE.into()
					}
				),
				4000
			));
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			assert_eq!(
				ParaBalances::free_balance(&PARAALICE.into()),
				INITIAL_BALANCE - 100
			);
		});

		ParaB::execute_with(|| {
			// free execution, full amount received
			assert_eq!(Assets::balance(1, &PARAALICE.into()), 100);
		});

		ParaB::execute_with(|| {
			// free execution, full amount received
			assert_ok!(XTokens::transfer(
				parachain::Origin::signed(PARAALICE.into()),
				1,
				100,
				X3(
					Junction::Parent,
					Junction::Parachain(1),
					Junction::AccountKey20 {
						network: NetworkId::Any,
						key: PARAALICE.into()
					}
				),
				4000
			));
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			assert_eq!(
				ParaBalances::free_balance(&PARAALICE.into()),
				INITIAL_BALANCE
			);
		});
	}
}