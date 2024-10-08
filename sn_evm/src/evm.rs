// Copyright 2024 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use evmlib::common::TxHash;
use libp2p::identity::PublicKey;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};

use crate::PaymentQuote;

/// The proof of payment for a data payment
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ProofOfPayment {
    /// The Quote we're paying for
    pub quote: PaymentQuote,
    /// The transaction hash
    pub tx_hash: TxHash,
}

impl ProofOfPayment {
    pub fn to_peer_id_payee(&self) -> Option<PeerId> {
        let pub_key = PublicKey::try_decode_protobuf(&self.quote.pub_key).ok()?;
        Some(PeerId::from_public_key(&pub_key))
    }
}
