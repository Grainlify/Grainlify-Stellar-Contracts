#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _, Ledger},
    token, Address, Env, IntoVal, TryFromVal, Val, Vec,
};

struct Fixture<'a> {
    env: Env,
    contract_id: Address,
    admin: Address,
    depositor: Address,
    recipient: Address,
    token_id: Address,
    client: BountyEscrowContractClient<'a>,
}

impl<'a> Fixture<'a> {
    fn new(claim_window: u64) -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(100);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();

        client.init(&admin, &token_id);
        client.set_claim_window(&claim_window);

        Self {
            env,
            contract_id,
            admin,
            depositor,
            recipient,
            token_id,
            client,
        }
    }

    fn authorize(&self, bounty_id: u64, amount: i128) {
        token::StellarAssetClient::new(&self.env, &self.token_id).mint(&self.depositor, &amount);
        self.client.lock_funds(
            &self.depositor,
            &bounty_id,
            &amount,
            &(self.env.ledger().timestamp() + 3_600),
        );
        self.client.authorize_claim(&bounty_id, &self.recipient);
    }

    fn last_dispute_event(&self) -> DisputeResolved {
        let events = self.env.events().all();
        assert!(events.len() > 0, "expected at least one contract event");

        let (contract_id, topics, data) = events.get(events.len() - 1).unwrap();
        let expected_topics: Vec<Val> =
            (symbol_short!("dispute"), symbol_short!("resolved")).into_val(&self.env);

        assert_eq!(contract_id, self.contract_id);
        assert_eq!(topics, expected_topics);

        DisputeResolved::try_from_val(&self.env, &data)
            .expect("dispute event payload should decode")
    }
}

#[test]
fn claim_emits_claimed_dispute_resolution() {
    let fixture = Fixture::new(60);
    fixture.authorize(11, 1_000);

    fixture.client.claim(&11);

    assert_eq!(
        fixture.last_dispute_event(),
        DisputeResolved {
            version: EVENT_VERSION_V2,
            bounty_id: 11,
            outcome: DisputeOutcome::Claimed,
            resolver: fixture.recipient.clone(),
            recipient: fixture.recipient.clone(),
            amount: 1_000,
            resolved_at: 100,
        }
    );
}

#[test]
fn manual_cancellation_emits_cancelled_dispute_resolution() {
    let fixture = Fixture::new(60);
    fixture.authorize(12, 2_000);

    fixture.client.cancel_pending_claim(&12);

    assert_eq!(
        fixture.last_dispute_event(),
        DisputeResolved {
            version: EVENT_VERSION_V2,
            bounty_id: 12,
            outcome: DisputeOutcome::Cancelled,
            resolver: fixture.admin.clone(),
            recipient: fixture.recipient.clone(),
            amount: 2_000,
            resolved_at: 100,
        }
    );
}

#[test]
fn expired_claim_emits_expired_dispute_resolution() {
    let fixture = Fixture::new(10);
    fixture.authorize(13, 3_000);
    fixture.env.ledger().set_timestamp(111);

    fixture.client.cancel_pending_claim(&13);

    assert_eq!(
        fixture.last_dispute_event(),
        DisputeResolved {
            version: EVENT_VERSION_V2,
            bounty_id: 13,
            outcome: DisputeOutcome::Expired,
            resolver: fixture.admin.clone(),
            recipient: fixture.recipient.clone(),
            amount: 3_000,
            resolved_at: 111,
        }
    );
}
