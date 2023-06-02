// Copyright (c) RoochNetwork
// SPDX-License-Identifier: Apache-2.0

use std::debug_assert;

use super::{
    authenticator::Authenticator, AbstractTransaction, AuthenticatorInfo, AuthenticatorResult,
    TransactionType,
};
use crate::address::RoochAddress;

use crate::H256;
use anyhow::Result;

use moveos_types::transaction::{AuthenticatableTransaction, MoveAction, MoveOSTransaction};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoochTransactionData {
    /// Sender's address.
    pub sender: RoochAddress,
    // Sequence number of this transaction corresponding to sender's account.
    pub sequence_number: u64,
    // The MoveAction to execute.
    pub action: MoveAction,
    //TODO how to define Gas paramter and AppID(Or ChainID)
}

impl RoochTransactionData {
    pub fn new(sender: RoochAddress, sequence_number: u64, action: MoveAction) -> Self {
        Self {
            sender,
            sequence_number,
            action,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        bcs::to_bytes(self).expect("encode transaction should success")
    }

    pub fn hash(&self) -> H256 {
        moveos_types::h256::sha3_256_of(self.encode().as_slice())
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoochTransaction {
    data: RoochTransactionData,
    authenticator: Authenticator,
}

impl RoochTransaction {
    pub fn new(data: RoochTransactionData, authenticator: Authenticator) -> Self {
        Self {
            data,
            authenticator,
        }
    }

    pub fn sender(&self) -> RoochAddress {
        self.data.sender
    }

    pub fn sequence_number(&self) -> u64 {
        self.data.sequence_number
    }

    pub fn action(&self) -> &MoveAction {
        &self.data.action
    }

    //TODO use protest Arbitrary to generate mock data
    #[cfg(test)]
    pub fn mock() -> RoochTransaction {
        use crate::address::RoochSupportedAddress;
        use crate::crypto::Signature;
        use fastcrypto::ed25519::Ed25519KeyPair;
        use fastcrypto::traits::KeyPair;
        use move_core_types::{
            account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
        };
        use moveos_types::move_types::FunctionId;

        let sender = RoochAddress::random();
        let sequence_number = 0;
        let payload = MoveAction::new_function_call(
            FunctionId::new(
                ModuleId::new(AccountAddress::random(), Identifier::new("test").unwrap()),
                Identifier::new("test").unwrap(),
            ),
            vec![],
            vec![],
        );

        let transaction_data = RoochTransactionData::new(sender, sequence_number, payload);
        let mut rng = rand::thread_rng();
        let ed25519_keypair: Ed25519KeyPair = Ed25519KeyPair::generate(&mut rng);
        let auth =
            Signature::new_hashed(transaction_data.hash().as_bytes(), &ed25519_keypair).into();
        RoochTransaction::new(transaction_data, auth)
    }
}

impl From<RoochTransaction> for MoveOSTransaction {
    fn from(tx: RoochTransaction) -> Self {
        let tx_hash = tx.tx_hash();
        MoveOSTransaction::new(tx.data.sender.into(), tx.data.action, tx_hash)
    }
}

impl AbstractTransaction for RoochTransaction {
    type Hash = H256;

    fn transaction_type(&self) -> super::TransactionType {
        TransactionType::Rooch
    }

    fn decode(bytes: &[u8]) -> Result<Self>
    where
        Self: std::marker::Sized,
    {
        bcs::from_bytes::<Self>(bytes).map_err(Into::into)
    }

    fn encode(&self) -> Vec<u8> {
        bcs::to_bytes(self).expect("encode transaction should success")
    }
}

impl AuthenticatableTransaction for RoochTransaction {
    type AuthenticatorInfo = AuthenticatorInfo;
    type AuthenticatorResult = AuthenticatorResult;

    //TODO unify the hash function
    fn tx_hash(&self) -> H256 {
        //TODO cache the hash
        self.data.hash()
    }

    fn authenticator_info(&self) -> AuthenticatorInfo {
        AuthenticatorInfo {
            sender: self.sender().into(),
            seqence_number: self.sequence_number(),
            authenticator: self.authenticator.clone(),
        }
    }

    fn construct_moveos_transaction(
        &self,
        result: AuthenticatorResult,
    ) -> Result<MoveOSTransaction> {
        debug_assert!(self.sender() == RoochAddress::from(result.resolved_address));
        Ok(self.clone().into())
    }
}