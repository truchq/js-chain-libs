#[macro_use]
mod transaction;
#[macro_use]
mod utils;

use bech32::{Bech32, ToBase32 as _};
use chain::{account, certificate, fee, key, transaction as tx, value};
use chain_core::property::Block as _;
use chain_core::property::Deserialize as _;
use chain_core::property::Fragment as _;
use chain_core::property::Serialize;
use chain_crypto as crypto;
use chain_impl_mockchain as chain;
use crypto::bech32::Bech32 as _;
use hex;
use js_sys::Uint8Array;
use rand_os::OsRng;
use std::convert::TryFrom;
use std::ops::{Add, Sub};
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use chain_core::mempack::{ReadBuf, Readable};

pub use transaction::*;

#[wasm_bindgen]
pub struct Bip32PrivateKey(crypto::SecretKey<crypto::Ed25519Bip32>);

#[wasm_bindgen]
impl Bip32PrivateKey {
    /// derive this private key with the given index.
    ///
    /// # Security considerations
    ///
    /// * hard derivation index cannot be soft derived with the public key
    ///
    /// # Hard derivation vs Soft derivation
    ///
    /// If you pass an index below 0x80000000 then it is a soft derivation.
    /// The advantage of soft derivation is that it is possible to derive the
    /// public key too. I.e. derivation the private key with a soft derivation
    /// index and then retrieving the associated public key is equivalent to
    /// deriving the public key associated to the parent private key.
    ///
    /// Hard derivation index does not allow public key derivation.
    ///
    /// This is why deriving the private key should not fail while deriving
    /// the public key may fail (if the derivation index is invalid).
    ///
    pub fn derive(&self, index: u32) -> Bip32PrivateKey {
        Bip32PrivateKey(crypto::derive::derive_sk_ed25519(&self.0, index))
    }

    pub fn generate_ed25519_bip32() -> Result<Bip32PrivateKey, JsValue> {
        OsRng::new()
            .map(crypto::SecretKey::<crypto::Ed25519Bip32>::generate)
            .map(Bip32PrivateKey)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
    }

    pub fn to_raw_key(&self) -> PrivateKey {
        PrivateKey(key::EitherEd25519SecretKey::Extended(
            crypto::derive::to_raw_sk(&self.0),
        ))
    }

    pub fn to_public(&self) -> Bip32PublicKey {
        Bip32PublicKey(self.0.to_public().into())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Bip32PrivateKey, JsValue> {
        crypto::SecretKey::<crypto::Ed25519Bip32>::from_binary(bytes)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(Bip32PrivateKey)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.as_ref().to_vec()
    }

    pub fn from_bech32(bech32_str: &str) -> Result<Bip32PrivateKey, JsValue> {
        crypto::SecretKey::try_from_bech32_str(&bech32_str)
            .map(Bip32PrivateKey)
            .map_err(|_| JsValue::from_str("Invalid secret key"))
    }

    pub fn to_bech32(&self) -> String {
        self.0.to_bech32_str()
    }

    pub fn from_bip39_entropy(entropy: &[u8], password: &[u8]) -> Bip32PrivateKey {
        Bip32PrivateKey(crypto::derive::from_bip39_entropy(&entropy, &password))
    }
}

#[wasm_bindgen]
pub struct Bip32PublicKey(crypto::PublicKey<crypto::Ed25519Bip32>);

#[wasm_bindgen]
impl Bip32PublicKey {
    /// derive this public key with the given index.
    ///
    /// # Errors
    ///
    /// If the index is not a soft derivation index (< 0x80000000) then
    /// calling this method will fail.
    ///
    /// # Security considerations
    ///
    /// * hard derivation index cannot be soft derived with the public key
    ///
    /// # Hard derivation vs Soft derivation
    ///
    /// If you pass an index below 0x80000000 then it is a soft derivation.
    /// The advantage of soft derivation is that it is possible to derive the
    /// public key too. I.e. derivation the private key with a soft derivation
    /// index and then retrieving the associated public key is equivalent to
    /// deriving the public key associated to the parent private key.
    ///
    /// Hard derivation index does not allow public key derivation.
    ///
    /// This is why deriving the private key should not fail while deriving
    /// the public key may fail (if the derivation index is invalid).
    ///
    pub fn derive(&self, index: u32) -> Result<Bip32PublicKey, JsValue> {
        crypto::derive::derive_pk_ed25519(&self.0, index)
            .map(Bip32PublicKey)
            .map_err(|e| JsValue::from_str(&format! {"{:?}", e}))
    }

    pub fn to_raw_key(&self) -> PublicKey {
        PublicKey(crypto::derive::to_raw_pk(&self.0))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Bip32PublicKey, JsValue> {
        crypto::PublicKey::<crypto::Ed25519Bip32>::from_binary(bytes)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(Bip32PublicKey)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.as_ref().to_vec()
    }

    pub fn from_bech32(bech32_str: &str) -> Result<Bip32PublicKey, JsValue> {
        crypto::PublicKey::try_from_bech32_str(&bech32_str)
            .map(Bip32PublicKey)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
    }

    pub fn to_bech32(&self) -> String {
        self.0.to_bech32_str()
    }
}

macro_rules! impl_signature {
    ($name:ident, $signee_type:ty, $verifier_type:ty) => {
        #[wasm_bindgen]
        pub struct $name(crypto::Signature<$signee_type, $verifier_type>);

        #[wasm_bindgen]
        impl $name {
            pub fn as_bytes(&self) -> Vec<u8> {
                self.0.as_ref().to_vec()
            }

            pub fn to_bech32(&self) -> String {
                self.0.to_bech32_str()
            }

            pub fn to_hex(&self) -> String {
                hex::encode(&self.0.as_ref())
            }

            pub fn from_bytes(bytes: &[u8]) -> Result<$name, JsValue> {
                crypto::Signature::from_binary(bytes)
                    .map($name)
                    .map_err(|e| JsValue::from_str(&format!("{}", e)))
            }

            pub fn from_bech32(bech32_str: &str) -> Result<$name, JsValue> {
                crypto::Signature::try_from_bech32_str(&bech32_str)
                    .map($name)
                    .map_err(|e| JsValue::from_str(&format!("{}", e)))
            }

            pub fn from_hex(input: &str) -> Result<$name, JsValue> {
                crypto::Signature::from_str(input)
                    .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
                    .map($name)
            }
        }
    };
}

#[wasm_bindgen]
pub struct LegacyDaedalusPrivateKey(crypto::SecretKey<crypto::LegacyDaedalus>);

#[wasm_bindgen]
impl LegacyDaedalusPrivateKey {
    pub fn from_bytes(bytes: &[u8]) -> Result<LegacyDaedalusPrivateKey, JsValue> {
        crypto::SecretKey::<crypto::LegacyDaedalus>::from_binary(bytes)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(LegacyDaedalusPrivateKey)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.as_ref().to_vec()
    }
}

impl_signature!(Ed25519Signature, Vec<u8>, crypto::Ed25519);
impl_signature!(AccountWitness, tx::WitnessAccountData, crypto::Ed25519);
impl_signature!(UtxoWitness, tx::WitnessUtxoData, crypto::Ed25519);
impl_signature!(LegacyUtxoWitness, tx::WitnessUtxoData, crypto::Ed25519Bip32);

/// ED25519 signing key, either normal or extended
#[wasm_bindgen]
pub struct PrivateKey(key::EitherEd25519SecretKey);

impl From<key::EitherEd25519SecretKey> for PrivateKey {
    fn from(secret_key: key::EitherEd25519SecretKey) -> PrivateKey {
        PrivateKey(secret_key)
    }
}

#[wasm_bindgen]
impl PrivateKey {
    /// Get private key from its bech32 representation
    /// ```javascript
    /// PrivateKey.from_bech32(&#39;ed25519_sk1ahfetf02qwwg4dkq7mgp4a25lx5vh9920cr5wnxmpzz9906qvm8qwvlts0&#39;);
    /// ```
    /// For an extended 25519 key
    /// ```javascript
    /// PrivateKey.from_bech32(&#39;ed25519e_sk1gqwl4szuwwh6d0yk3nsqcc6xxc3fpvjlevgwvt60df59v8zd8f8prazt8ln3lmz096ux3xvhhvm3ca9wj2yctdh3pnw0szrma07rt5gl748fp&#39;);
    /// ```
    pub fn from_bech32(bech32_str: &str) -> Result<PrivateKey, JsValue> {
        crypto::SecretKey::try_from_bech32_str(&bech32_str)
            .map(key::EitherEd25519SecretKey::Extended)
            .or_else(|_| {
                crypto::SecretKey::try_from_bech32_str(&bech32_str)
                    .map(key::EitherEd25519SecretKey::Normal)
            })
            .map(PrivateKey)
            .map_err(|_| JsValue::from_str("Invalid secret key"))
    }

    pub fn to_public(&self) -> PublicKey {
        self.0.to_public().into()
    }

    pub fn generate_ed25519() -> Result<PrivateKey, JsValue> {
        OsRng::new()
            .map(crypto::SecretKey::<crypto::Ed25519>::generate)
            .map(key::EitherEd25519SecretKey::Normal)
            .map(PrivateKey)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
    }

    pub fn generate_ed25519extended() -> Result<PrivateKey, JsValue> {
        OsRng::new()
            .map(crypto::SecretKey::<crypto::Ed25519Extended>::generate)
            .map(key::EitherEd25519SecretKey::Extended)
            .map(PrivateKey)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
    }

    pub fn to_bech32(&self) -> String {
        match self.0 {
            key::EitherEd25519SecretKey::Normal(ref secret) => secret.to_bech32_str(),
            key::EitherEd25519SecretKey::Extended(ref secret) => secret.to_bech32_str(),
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match self.0 {
            key::EitherEd25519SecretKey::Normal(ref secret) => secret.as_ref().to_vec(),
            key::EitherEd25519SecretKey::Extended(ref secret) => secret.as_ref().to_vec(),
        }
    }

    pub fn from_extended_bytes(bytes: &[u8]) -> Result<PrivateKey, JsValue> {
        crypto::SecretKey::from_binary(bytes)
            .map(key::EitherEd25519SecretKey::Extended)
            .map(PrivateKey)
            .map_err(|_| JsValue::from_str("Invalid extended secret key"))
    }

    pub fn from_normal_bytes(bytes: &[u8]) -> Result<PrivateKey, JsValue> {
        crypto::SecretKey::from_binary(bytes)
            .map(key::EitherEd25519SecretKey::Normal)
            .map(PrivateKey)
            .map_err(|_| JsValue::from_str("Invalid normal secret key"))
    }

    pub fn sign(&self, message: &[u8]) -> Ed25519Signature {
        Ed25519Signature(self.0.sign(&message.to_vec()))
    }
}

/// ED25519 key used as public key
#[wasm_bindgen]
#[derive(Clone)]
pub struct PublicKey(crypto::PublicKey<crypto::Ed25519>);

impl From<crypto::PublicKey<crypto::Ed25519>> for PublicKey {
    fn from(key: crypto::PublicKey<crypto::Ed25519>) -> PublicKey {
        PublicKey(key)
    }
}

#[wasm_bindgen]
impl PublicKey {
    /// Get private key from its bech32 representation
    /// Example:
    /// ```javascript
    /// const pkey = PublicKey.from_bech32(&#39;ed25519_pk1dgaagyh470y66p899txcl3r0jaeaxu6yd7z2dxyk55qcycdml8gszkxze2&#39;);
    /// ```
    pub fn from_bech32(bech32_str: &str) -> Result<PublicKey, JsValue> {
        crypto::PublicKey::try_from_bech32_str(&bech32_str)
            .map(PublicKey)
            .map_err(|_| JsValue::from_str("Malformed public key"))
    }

    pub fn to_bech32(&self) -> String {
        self.0.to_bech32_str()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.as_ref().to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<PublicKey, JsValue> {
        crypto::PublicKey::from_binary(bytes)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(PublicKey)
    }

    pub fn verify(&self, data: &[u8], signature: &Ed25519Signature) -> bool {
        signature.0.verify_slice(&self.0, data) == crypto::Verification::Success
    }
}

#[wasm_bindgen]
pub struct PublicKeys(Vec<PublicKey>);

#[wasm_bindgen]
impl PublicKeys {
    #[wasm_bindgen(constructor)]
    pub fn new() -> PublicKeys {
        PublicKeys(vec![])
    }

    pub fn size(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, index: usize) -> PublicKey {
        self.0[index].clone()
    }

    pub fn add(&mut self, key: &PublicKey) {
        self.0.push(key.clone());
    }
}

//-----------------------------//
//----------Address------------//
//-----------------------------//

#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SingleAddress(
    chain_addr::Discrimination,
    crypto::PublicKey<crypto::Ed25519>,
);

#[wasm_bindgen]
impl SingleAddress {
    pub fn get_spending_key(&self) -> PublicKey {
        PublicKey::from(self.1.clone())
    }

    pub fn to_base_address(&self) -> Address {
        Address::single_from_public_key(&self.get_spending_key(), self.0.into())
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupAddress(
    chain_addr::Discrimination,
    crypto::PublicKey<crypto::Ed25519>,
    crypto::PublicKey<crypto::Ed25519>,
);

#[wasm_bindgen]
impl GroupAddress {
    pub fn get_spending_key(&self) -> PublicKey {
        PublicKey::from(self.1.clone())
    }
    pub fn get_account_key(&self) -> PublicKey {
        PublicKey::from(self.2.clone())
    }
    pub fn to_base_address(&self) -> Address {
        Address::delegation_from_public_key(
            &self.get_spending_key(),
            &self.get_account_key(),
            self.0.into(),
        )
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountAddress(
    chain_addr::Discrimination,
    crypto::PublicKey<crypto::Ed25519>,
);

#[wasm_bindgen]
impl AccountAddress {
    pub fn get_account_key(&self) -> PublicKey {
        PublicKey::from(self.1.clone())
    }
    pub fn to_base_address(&self) -> Address {
        Address::account_from_public_key(&self.get_account_key(), self.0.into())
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultisigAddress(chain_addr::Discrimination, [u8; 32]);

#[wasm_bindgen]
impl MultisigAddress {
    pub fn get_merkle_root(&self) -> Vec<u8> {
        self.1.to_vec()
    }
    pub fn to_base_address(&self) -> Result<Address, JsValue> {
        Address::multisig_from_merkle_root(self.get_merkle_root().as_slice(), self.0.into())
    }
}

/// An address of any type, this can be one of
/// * A utxo-based address without delegation (single)
/// * A utxo-based address with delegation (group)
/// * An address for an account
#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Address(chain_addr::Address);

#[wasm_bindgen]
impl Address {
    pub fn from_bytes(bytes: Uint8Array) -> Result<Address, JsValue> {
        let mut slice: Box<[u8]> = vec![0; bytes.length() as usize].into_boxed_slice();
        bytes.copy_to(&mut *slice);
        chain_addr::Address::deserialize(&*slice)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(Address)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.serialize_as_vec().unwrap()
    }

    //XXX: Maybe this should be from_bech32?
    /// Construct Address from its bech32 representation
    /// Example
    /// ```javascript
    /// const address = Address.from_string(&#39;ca1q09u0nxmnfg7af8ycuygx57p5xgzmnmgtaeer9xun7hly6mlgt3pjyknplu&#39;);
    /// ```
    pub fn from_string(s: &str) -> Result<Address, JsValue> {
        chain_addr::AddressReadable::from_string_anyprefix(s)
            .map(|address_readable| Address(address_readable.to_address()))
            .map_err(|e| JsValue::from_str(&format! {"{:?}", e}))
    }

    /// Get Address bech32 (string) representation with a given prefix
    /// ```javascript
    /// let public_key = PublicKey.from_bech32(
    ///     &#39;ed25519_pk1kj8yvfrh5tg7n62kdcw3kw6zvtcafgckz4z9s6vc608pzt7exzys4s9gs8&#39;
    /// );
    /// let discriminant = AddressDiscrimination.Test;
    /// let address = Address.single_from_public_key(public_key, discriminant);
    /// address.to_string(&#39;ta&#39;)
    /// // ta1sj6gu33yw73dr60f2ehp6xemgf30r49rzc25gkrfnrfuuyf0mycgnj78ende550w5njvwzyr20q6rypdea597uu3jnwfltljddl59cseaq7yn9
    /// ```
    pub fn to_string(&self, prefix: &str) -> String {
        format!(
            "{}",
            chain_addr::AddressReadable::from_address(prefix, &self.0)
        )
    }

    /// Construct a single non-account address from a public key
    /// ```javascript
    /// let public_key = PublicKey.from_bech32(
    ///     &#39;ed25519_pk1kj8yvfrh5tg7n62kdcw3kw6zvtcafgckz4z9s6vc608pzt7exzys4s9gs8&#39;
    /// );
    /// let address = Address.single_from_public_key(public_key, AddressDiscrimination.Test);
    /// ```
    pub fn single_from_public_key(
        key: &PublicKey,
        discrimination: AddressDiscrimination,
    ) -> Address {
        chain_addr::Address(
            discrimination.into(),
            chain_addr::Kind::Single(key.0.clone()),
        )
        .into()
    }

    /// Construct a non-account address from a pair of public keys, delegating founds from the first to the second
    pub fn delegation_from_public_key(
        key: &PublicKey,
        delegation: &PublicKey,
        discrimination: AddressDiscrimination,
    ) -> Address {
        chain_addr::Address(
            discrimination.into(),
            chain_addr::Kind::Group(key.0.clone(), delegation.0.clone()),
        )
        .into()
    }

    /// Construct address of account type from a public key
    pub fn account_from_public_key(
        key: &PublicKey,
        discrimination: AddressDiscrimination,
    ) -> Address {
        chain_addr::Address(
            discrimination.into(),
            chain_addr::Kind::Account(key.0.clone()),
        )
        .into()
    }

    pub fn multisig_from_merkle_root(
        merkle_root: &[u8],
        discrimination: AddressDiscrimination,
    ) -> Result<Address, JsValue> {
        match merkle_root.len() {
            32 => {
                let mut sized_root = [0; 32];
                sized_root.copy_from_slice(&merkle_root);
                Ok(chain_addr::Address(
                    discrimination.into(),
                    chain_addr::Kind::Multisig(sized_root),
                )
                .into())
            }
            _ => Err(JsValue::from_str("Invalid merkle root size")),
        }
    }

    pub fn get_discrimination(&self) -> AddressDiscrimination {
        AddressDiscrimination::from(self.0.discrimination())
    }

    pub fn get_kind(&self) -> AddressKind {
        AddressKind::from(self.0.to_kind_type())
    }

    pub fn to_single_address(&self) -> Option<SingleAddress> {
        match self.0.kind() {
            chain_addr::Kind::Single(ref spending_key) => {
                Some(SingleAddress(self.0.discrimination(), spending_key.clone()))
            }
            _ => None,
        }
    }

    pub fn to_group_address(&self) -> Option<GroupAddress> {
        match self.0.kind() {
            chain_addr::Kind::Group(ref spending_key, ref account_key) => Some(GroupAddress(
                self.0.discrimination(),
                spending_key.clone(),
                account_key.clone(),
            )),
            _ => None,
        }
    }

    pub fn to_account_address(&self) -> Option<AccountAddress> {
        match self.0.kind() {
            chain_addr::Kind::Account(ref account_key) => {
                Some(AccountAddress(self.0.discrimination(), account_key.clone()))
            }
            _ => None,
        }
    }

    pub fn to_multisig_address(&self) -> Option<MultisigAddress> {
        match self.0.kind() {
            chain_addr::Kind::Multisig(ref merkle_root) => Some(MultisigAddress(
                self.0.discrimination(),
                merkle_root.clone(),
            )),
            _ => None,
        }
    }
}

impl From<chain_addr::Address> for Address {
    fn from(address: chain_addr::Address) -> Address {
        Address(address)
    }
}

/// Allow to differentiate between address in
/// production and testing setting, so that
/// one type of address is not used in another setting.
/// Example
/// ```javascript
/// let discriminant = AddressDiscrimination.Test;
/// let address = Address::single_from_public_key(public_key, discriminant);
/// ```
#[wasm_bindgen]
pub enum AddressDiscrimination {
    Production,
    Test,
}

impl Into<chain_addr::Discrimination> for AddressDiscrimination {
    fn into(self) -> chain_addr::Discrimination {
        match self {
            AddressDiscrimination::Production => chain_addr::Discrimination::Production,
            AddressDiscrimination::Test => chain_addr::Discrimination::Test,
        }
    }
}
impl From<chain_addr::Discrimination> for AddressDiscrimination {
    fn from(discrimination: chain_addr::Discrimination) -> Self {
        match discrimination {
            chain_addr::Discrimination::Production => AddressDiscrimination::Production,
            chain_addr::Discrimination::Test => AddressDiscrimination::Test,
        }
    }
}

#[wasm_bindgen]
pub enum AddressKind {
    Single,
    Group,
    Account,
    Multisig,
}

impl Into<chain_addr::KindType> for AddressKind {
    fn into(self) -> chain_addr::KindType {
        match self {
            AddressKind::Single => chain_addr::KindType::Single,
            AddressKind::Group => chain_addr::KindType::Group,
            AddressKind::Account => chain_addr::KindType::Account,
            AddressKind::Multisig => chain_addr::KindType::Multisig,
        }
    }
}
impl From<chain_addr::KindType> for AddressKind {
    fn from(kind: chain_addr::KindType) -> Self {
        match kind {
            chain_addr::KindType::Single => AddressKind::Single,
            chain_addr::KindType::Group => AddressKind::Group,
            chain_addr::KindType::Account => AddressKind::Account,
            chain_addr::KindType::Multisig => AddressKind::Multisig,
        }
    }
}

impl_collection!(Outputs, Output);
impl_collection!(Inputs, Input);
impl_collection!(Fragments, Fragment);

/// Helper to add change addresses when finalizing a transaction, there are currently two options
/// * forget: use all the excess money as fee
/// * one: send all the excess money to the given address
#[wasm_bindgen]
pub struct OutputPolicy(tx::OutputPolicy);

impl From<tx::OutputPolicy> for OutputPolicy {
    fn from(output_policy: tx::OutputPolicy) -> OutputPolicy {
        OutputPolicy(output_policy)
    }
}

#[wasm_bindgen]
impl OutputPolicy {
    /// don't do anything with the excess money in transaction
    pub fn forget() -> OutputPolicy {
        tx::OutputPolicy::Forget.into()
    }

    /// use the given address as the only change address
    pub fn one(address: &Address) -> OutputPolicy {
        tx::OutputPolicy::One(address.0.clone()).into()
    }
}

/// Type for representing the hash of a Transaction, necessary for signing it
#[wasm_bindgen]
pub struct TransactionSignDataHash(tx::TransactionSignDataHash);

#[wasm_bindgen]
impl TransactionSignDataHash {
    pub fn from_bytes(bytes: &[u8]) -> Result<TransactionSignDataHash, JsValue> {
        tx::TransactionSignDataHash::try_from(bytes)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(|digest| digest.into())
    }

    pub fn from_hex(input: &str) -> Result<TransactionSignDataHash, JsValue> {
        tx::TransactionSignDataHash::from_str(input)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
            .map(TransactionSignDataHash)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.as_ref().to_vec()
    }
}

impl From<tx::TransactionSignDataHash> for TransactionSignDataHash {
    fn from(txid: tx::TransactionSignDataHash) -> TransactionSignDataHash {
        TransactionSignDataHash(txid)
    }
}

/// Type for representing a generic Hash
#[wasm_bindgen]
pub struct Hash(key::Hash);

impl From<key::Hash> for Hash {
    fn from(hash: key::Hash) -> Hash {
        Hash(hash)
    }
}

#[wasm_bindgen]
impl Hash {
    pub fn calculate(bytes: &[u8]) -> Hash {
        key::Hash::hash_bytes(bytes).into()
    }

    pub fn from_hex(hex_string: &str) -> Result<Hash, JsValue> {
        key::Hash::from_str(hex_string)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(Hash)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.serialize_as_vec().unwrap()
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct Input(tx::Input);

impl From<tx::Input> for Input {
    fn from(input: tx::Input) -> Input {
        Input(input)
    }
}

#[wasm_bindgen]
pub enum InputKind {
    Account,
    Utxo,
}

/// Generalized input which have a specific input value, and
/// either contains an account reference or a TransactionSignDataHash+index
///
/// This uniquely refer to a specific source of value.
#[wasm_bindgen]
impl Input {
    pub fn from_utxo(utxo_pointer: &UtxoPointer) -> Self {
        Input(tx::Input::from_utxo(utxo_pointer.0))
    }

    pub fn from_account(account: &Account, v: &Value) -> Self {
        let identifier = account.to_identifier();
        Input(tx::Input::from_account(identifier.0, v.0))
    }

    pub fn get_type(&self) -> InputKind {
        match self.0.get_type() {
            tx::InputType::Account => InputKind::Account,
            tx::InputType::Utxo => InputKind::Utxo,
        }
    }

    pub fn is_account(&self) -> bool {
        match self.0.get_type() {
            tx::InputType::Account => true,
            _ => false,
        }
    }

    pub fn is_utxo(&self) -> bool {
        match self.0.get_type() {
            tx::InputType::Utxo => true,
            _ => false,
        }
    }

    pub fn value(&self) -> Value {
        self.0.value().into()
    }

    /// Get the inner UtxoPointer if the Input type is Utxo
    pub fn get_utxo_pointer(&self) -> Result<UtxoPointer, JsValue> {
        match self.0.to_enum() {
            tx::InputEnum::UtxoInput(utxo_pointer) => Ok(utxo_pointer.into()),
            _ => Err(JsValue::from_str("Input is not from utxo")),
        }
    }

    /// Get the source Account if the Input type is Account
    pub fn get_account_identifier(&self) -> Result<AccountIdentifier, JsValue> {
        match self.0.to_enum() {
            tx::InputEnum::AccountInput(account, _) => Ok(AccountIdentifier(account)),
            _ => Err(JsValue::from_str("Input is not from account")),
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.bytes().to_vec()
    }

    pub fn from_bytes(bytes: Uint8Array) -> Result<Input, JsValue> {
        let mut slice: Box<[u8]> = vec![0; bytes.length() as usize].into_boxed_slice();
        bytes.copy_to(&mut *slice);
        tx::Input::deserialize(&*slice)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(Input)
    }
}

/// Unspent transaction pointer. This is composed of:
/// * the transaction identifier where the unspent output is (a FragmentId)
/// * the output index within the pointed transaction's outputs
/// * the value we expect to read from this output, this setting is added in order to protect undesired withdrawal
/// and to set the actual fee in the transaction.
#[wasm_bindgen]
#[derive(Debug)]
pub struct UtxoPointer(tx::UtxoPointer);

impl From<tx::UtxoPointer> for UtxoPointer {
    fn from(ptr: tx::UtxoPointer) -> UtxoPointer {
        UtxoPointer(ptr)
    }
}

#[wasm_bindgen]
impl UtxoPointer {
    pub fn new(fragment_id: &FragmentId, output_index: u8, value: &Value) -> UtxoPointer {
        UtxoPointer(tx::UtxoPointer {
            transaction_id: fragment_id.0.clone(),
            output_index,
            value: value.0,
        })
    }

    pub fn output_index(&self) -> u8 {
        self.0.output_index
    }

    pub fn fragment_id(&self) -> FragmentId {
        self.0.transaction_id.into()
    }
}

/// This is either an single account or a multisig account depending on the witness type
#[wasm_bindgen]
#[derive(Debug)]
pub struct Account(tx::AccountIdentifier);

impl From<tx::AccountIdentifier> for Account {
    fn from(account_identifier: tx::AccountIdentifier) -> Account {
        Account(account_identifier)
    }
}

#[wasm_bindgen]
impl Account {
    pub fn from_address(address: &Address) -> Result<Account, JsValue> {
        match address.0.kind() {
            chain_addr::Kind::Account(key) => {
                Ok(Account(tx::AccountIdentifier::Single(key.clone().into())))
            }
            chain_addr::Kind::Multisig(id) => {
                Ok(Account(tx::AccountIdentifier::Multi(id.clone().into())))
            }
            _ => Err(JsValue::from_str("Address is not account")),
        }
    }

    pub fn to_address(&self, discriminant: AddressDiscrimination) -> Address {
        let kind = match &self.0 {
            tx::AccountIdentifier::Single(id) => chain_addr::Kind::Account(id.clone().into()),
            tx::AccountIdentifier::Multi(id) => {
                let mut bytes = [0u8; chain_crypto::hash::HASH_SIZE_256];
                bytes.copy_from_slice(id.as_ref());
                chain_addr::Kind::Multisig(bytes)
            }
        };
        chain_addr::Address(discriminant.into(), kind).into()
    }

    pub fn single_from_public_key(key: &PublicKey) -> Account {
        Account(tx::AccountIdentifier::Single(key.0.clone().into()))
    }

    pub fn to_identifier(&self) -> AccountIdentifier {
        let unspecified = match &self.0 {
            tx::AccountIdentifier::Single(id) => {
                tx::UnspecifiedAccountIdentifier::from_single_account(id.clone())
            }
            tx::AccountIdentifier::Multi(id) => {
                tx::UnspecifiedAccountIdentifier::from_multi_account(id.clone())
            }
        };

        AccountIdentifier(unspecified)
    }
}

#[wasm_bindgen]
pub struct AccountIdentifier(tx::UnspecifiedAccountIdentifier);

#[wasm_bindgen]
impl AccountIdentifier {
    pub fn to_hex(&self) -> String {
        hex::encode(self.0.as_ref())
    }

    pub fn to_account_single(&self) -> Result<Account, JsValue> {
        self.0
            .to_single_account()
            .ok_or(JsValue::from_str(
                "can't be used as a public key for single account",
            ))
            .map(|acc| Account(tx::AccountIdentifier::Single(acc)))
    }

    pub fn to_account_multi(&self) -> Account {
        Account(tx::AccountIdentifier::Multi(self.0.to_multi_account()))
    }
}

/// Type for representing a Transaction Output, composed of an Address and a Value
#[wasm_bindgen]
#[derive(Clone)]
pub struct Output(tx::Output<chain_addr::Address>);

impl From<tx::Output<chain_addr::Address>> for Output {
    fn from(output: tx::Output<chain_addr::Address>) -> Output {
        Output(output)
    }
}

#[wasm_bindgen]
impl Output {
    pub fn address(&self) -> Address {
        self.0.address.clone().into()
    }

    pub fn value(&self) -> Value {
        self.0.value.into()
    }
}

/// Type used for representing certain amount of lovelaces.
/// It wraps an unsigned 64 bits number.
/// Strings are used for passing to and from javascript,
/// as the native javascript Number type can't hold the entire u64 range
/// and BigInt is not yet implemented in all the browsers
#[wasm_bindgen]
#[derive(Debug, Eq, PartialEq)]
pub struct Value(value::Value);

impl AsRef<u64> for Value {
    fn as_ref(&self) -> &u64 {
        &self.0.as_ref()
    }
}

impl From<u64> for Value {
    fn from(number: u64) -> Value {
        value::Value(number).into()
    }
}

#[wasm_bindgen]
impl Value {
    /// Parse the given string into a rust u64 numeric type.
    pub fn from_str(s: &str) -> Result<Value, JsValue> {
        s.parse::<u64>()
            .map_err(|e| JsValue::from_str(&format! {"{:?}", e}))
            .map(|number| number.into())
    }

    /// Return the wrapped u64 formatted as a string.
    pub fn to_str(&self) -> String {
        format!("{}", self.0)
    }

    pub fn checked_add(&self, other: &Value) -> Result<Value, JsValue> {
        self.0
            .add(other.0)
            .map_err(|e| JsValue::from_str(&format!("{}", &format!("{}", e))))
            .map(Value)
    }

    pub fn checked_sub(&self, other: &Value) -> Result<Value, JsValue> {
        self.0
            .sub(other.0)
            .map_err(|e| JsValue::from_str(&format!("{}", &format!("{}", e))))
            .map(Value)
    }
}

impl From<value::Value> for Value {
    fn from(value: value::Value) -> Value {
        Value(value)
    }
}

#[wasm_bindgen]
pub struct U128(u128);

impl From<u128> for U128 {
    fn from(number: u128) -> U128 {
        U128(number)
    }
}

#[wasm_bindgen]
impl U128 {
    pub fn from_be_bytes(bytes: Uint8Array) -> Result<U128, JsValue> {
        if bytes.length() == std::mem::size_of::<u128>() as u32 {
            let mut slice = [0u8; 16];
            bytes.copy_to(&mut slice);
            Ok(u128::from_be_bytes(slice).into())
        } else {
            Err(JsValue::from_str(&format!(
                "Invalid array length. Found {}, expected: 16",
                bytes.length()
            )))
        }
    }

    pub fn from_le_bytes(bytes: Uint8Array) -> Result<U128, JsValue> {
        if bytes.length() == std::mem::size_of::<u128>() as u32 {
            let mut slice = [0u8; 16];
            bytes.copy_to(&mut slice);
            Ok(u128::from_le_bytes(slice).into())
        } else {
            Err(JsValue::from_str(&format!(
                "Invalid array length. Found {}, expected: 16",
                bytes.length()
            )))
        }
    }

    pub fn from_str(s: &str) -> Result<U128, JsValue> {
        s.parse::<u128>()
            .map_err(|e| JsValue::from_str(&format! {"{:?}", e}))
            .map(U128)
    }

    pub fn to_str(&self) -> String {
        format!("{}", self.0)
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct GenesisPraosLeaderHash(chain::certificate::GenesisPraosLeaderHash);

impl From<chain::certificate::GenesisPraosLeaderHash> for GenesisPraosLeaderHash {
    fn from(key_hash: chain::certificate::GenesisPraosLeaderHash) -> GenesisPraosLeaderHash {
        GenesisPraosLeaderHash(key_hash)
    }
}

#[wasm_bindgen]
impl GenesisPraosLeaderHash {
    pub fn from_hex(hex_string: &str) -> Result<GenesisPraosLeaderHash, JsValue> {
        crypto::Blake2b256::from_str(hex_string)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
            .map(|hash| GenesisPraosLeaderHash(hash.into()))
    }

    pub fn to_string(&self) -> String {
        format!("{}", self.0).to_string()
    }
}

#[wasm_bindgen]
pub struct GenesisPraosLeader(chain::leadership::genesis::GenesisPraosLeader);

impl From<chain::leadership::genesis::GenesisPraosLeader> for GenesisPraosLeader {
    fn from(leader: chain::leadership::genesis::GenesisPraosLeader) -> GenesisPraosLeader {
        GenesisPraosLeader(leader)
    }
}

#[wasm_bindgen]
impl GenesisPraosLeader {
    pub fn new(
        kes_public_key: &KesPublicKey,
        vrf_public_key: &VrfPublicKey,
    ) -> Self {
        Self(chain::leadership::genesis::GenesisPraosLeader {
            kes_public_key: kes_public_key.0.clone(),
            vrf_public_key: vrf_public_key.0.clone(),
        })
    }

}

#[wasm_bindgen]
pub struct Certificate(certificate::Certificate);

impl From<certificate::Certificate> for Certificate {
    fn from(certificate: certificate::Certificate) -> Certificate {
        Certificate(certificate)
    }
}

#[wasm_bindgen]
pub struct PoolRegistration(chain::certificate::PoolRegistration);

impl From<chain::certificate::PoolRegistration> for PoolRegistration {
    fn from(info: chain::certificate::PoolRegistration) -> PoolRegistration {
        PoolRegistration(info)
    }
}

#[wasm_bindgen]
pub struct StakeDelegation(chain::certificate::StakeDelegation);

impl From<chain::certificate::StakeDelegation> for StakeDelegation {
    fn from(info: chain::certificate::StakeDelegation) -> StakeDelegation {
        StakeDelegation(info)
    }
}

#[wasm_bindgen]
/// Set the choice of delegation:
///
/// * No delegation
/// * Full delegation of this account to a specific pool
/// * Ratio of stake to multiple pools
pub struct DelegationType(chain::account::DelegationType);

impl From<chain::account::DelegationType> for DelegationType {
    fn from(delegation: chain::account::DelegationType) -> DelegationType {
        DelegationType(delegation)
    }
}

#[wasm_bindgen]
pub enum DelegationKind {
    NonDelegated,
    Full,
    Ratio,
}

#[wasm_bindgen]
impl DelegationType {
    pub fn non_delegated() -> Self {
        Self(chain::account::DelegationType::NonDelegated)
    }

    pub fn full(pool_id: &PoolId) -> Self {
        Self(chain::account::DelegationType::Full(pool_id.0.clone()))
    }

    pub fn ratio(r: &DelegationRatio) -> Self {
        Self(chain::account::DelegationType::Ratio(r.0.clone()))
    }

    pub fn get_kind(&self) -> DelegationKind {
        match self.0 {
            chain::account::DelegationType::NonDelegated => DelegationKind::NonDelegated,
            chain::account::DelegationType::Full(_) => DelegationKind::Full,
            chain::account::DelegationType::Ratio(_) => DelegationKind::Ratio,
        }
    }

    pub fn get_full(&self) -> Option<PoolId> {
        match &self.0 {
            chain::account::DelegationType::Full(pool_id) => Some(pool_id.clone().into()),
            _ => None,
        }
    }
}

/// Delegation Ratio type express a number of parts
/// and a list of pools and their individual parts
///
/// E.g. parts: 7, pools: [(A,2), (B,1), (C,4)] means that
/// A is associated with 2/7 of the stake, B has 1/7 of stake and C
/// has 4/7 of the stake.
///
/// It's invalid to have less than 2 elements in the array,
/// and by extension parts need to be equal to the sum of individual
/// pools parts.
#[wasm_bindgen]
pub struct DelegationRatio(chain::account::DelegationRatio);

#[wasm_bindgen]
#[derive(Clone)]
pub struct PoolDelegationRatio {
    pool: PoolId,
    part: u8,
}

#[wasm_bindgen]
impl PoolDelegationRatio {
    //TODO: Add constructor attribute
    pub fn new(pool: &PoolId, part: u8) -> PoolDelegationRatio {
        Self {
            pool: pool.clone(),
            part,
        }
    }
}

impl_collection!(PoolDelegationRatios, PoolDelegationRatio);

#[wasm_bindgen]
impl DelegationRatio {
    //TODO: Add constructor attribute
    pub fn new(parts: u8, pools: &PoolDelegationRatios) -> Option<DelegationRatio> {
        let pools = pools
            .0
            .iter()
            .map(|PoolDelegationRatio { pool, part }| (pool.0.clone(), *part))
            .collect();

        // FIXME: It could be useful to return an error instea of an Option?
        chain::account::DelegationRatio::new(parts, pools).map(Self)
    }
}

#[wasm_bindgen]
impl StakeDelegation {
    /// Create a stake delegation object from account (stake key) to pool_id
    pub fn new(delegation_type: &DelegationType, account: &PublicKey) -> StakeDelegation {
        certificate::StakeDelegation {
            account_id: tx::UnspecifiedAccountIdentifier::from_single_account(
                account.0.clone().into(),
            ),
            delegation: delegation_type.0.clone(),
        }
        .into()
    }

    pub fn delegation_type(&self) -> DelegationType {
        self.0.delegation.clone().into()
    }

    pub fn account(&self) -> AccountIdentifier {
        AccountIdentifier(self.0.account_id.clone())
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.serialize().as_ref().to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<StakeDelegation, JsValue> {
        let mut buf = ReadBuf::from(&bytes);
        chain::certificate::StakeDelegation::read(&mut buf)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(StakeDelegation)
    }
}

#[wasm_bindgen]
pub struct OwnerStakeDelegation(chain::certificate::OwnerStakeDelegation);

impl From<chain::certificate::OwnerStakeDelegation> for OwnerStakeDelegation {
    fn from(info: chain::certificate::OwnerStakeDelegation) -> OwnerStakeDelegation {
        OwnerStakeDelegation(info)
    }
}

#[wasm_bindgen]
impl OwnerStakeDelegation {
    pub fn new(delegation_type: &DelegationType) -> OwnerStakeDelegation {
        certificate::OwnerStakeDelegation {
            delegation: delegation_type.0.clone(),
        }
        .into()
    }

    pub fn delegation_type(&self) -> DelegationType {
        self.0.delegation.clone().into()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.serialize().as_ref().to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<OwnerStakeDelegation, JsValue> {
        let mut buf = ReadBuf::from(&bytes);
        chain::certificate::OwnerStakeDelegation::read(&mut buf)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(OwnerStakeDelegation)
    }
}

#[wasm_bindgen]
pub struct PoolRetirement(chain::certificate::PoolRetirement);

impl From<chain::certificate::PoolRetirement> for PoolRetirement {
    fn from(retirement: chain::certificate::PoolRetirement) -> PoolRetirement {
        PoolRetirement(retirement)
    }
}

#[wasm_bindgen]
impl PoolRetirement {
    pub fn new(pool_id: &PoolId, retirement_time_offset: &TimeOffsetSeconds) -> Self {
        chain::certificate::PoolRetirement {
            pool_id: pool_id.0.clone(),
            retirement_time: retirement_time_offset.0,
        }
        .into()
    }

    pub fn pool_id(&self) -> PoolId {
        self.0.pool_id.clone().into()
    }

    pub fn retirement_time(&self) -> TimeOffsetSeconds {
        self.0.retirement_time.clone().into()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.serialize().as_ref().to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<PoolRetirement, JsValue> {
        let mut buf = ReadBuf::from(&bytes);
        chain::certificate::PoolRetirement::read(&mut buf)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(PoolRetirement)
    }
}

#[wasm_bindgen]
pub struct PoolUpdate(chain::certificate::PoolUpdate);

impl From<chain::certificate::PoolUpdate> for PoolUpdate {
    fn from(update: chain::certificate::PoolUpdate) -> PoolUpdate {
        PoolUpdate(update)
    }
}

#[wasm_bindgen]
impl PoolUpdate {
    pub fn new(
        pool_id: &PoolId,
        start_validity: &TimeOffsetSeconds,
        previous_keys: &GenesisPraosLeaderHash,
        updated_keys: &GenesisPraosLeader,
    ) -> Self {
        chain::certificate::PoolUpdate {
            pool_id: pool_id.0.clone(),
            start_validity: start_validity.0.clone(),
            previous_keys: previous_keys.0.clone(),
            updated_keys: updated_keys.0.clone(),
        }
        .into()
    }

    pub fn pool_id(&self) -> PoolId {
        self.0.pool_id.clone().into()
    }

    pub fn start_validity(&self) -> TimeOffsetSeconds {
        self.0.start_validity.into()
    }

    pub fn previous_keys(&self) -> GenesisPraosLeaderHash {
        GenesisPraosLeaderHash(self.0.previous_keys.clone())
    }

    pub fn updated_keys(&self) -> GenesisPraosLeader {
        GenesisPraosLeader(self.0.updated_keys.clone())
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.serialize().as_ref().to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<PoolUpdate, JsValue> {
        let mut buf = ReadBuf::from(&bytes);
        chain::certificate::PoolUpdate::read(&mut buf)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(PoolUpdate)
    }
}

#[wasm_bindgen]
pub enum CertificateKind {
    StakeDelegation,
    OwnerStakeDelegation,
    PoolRegistration,
    PoolRetirement,
    PoolUpdate,
}
#[wasm_bindgen]
impl Certificate {
    /// Create a Certificate for StakeDelegation
    pub fn stake_delegation(stake_delegation: &StakeDelegation) -> Certificate {
        certificate::Certificate::StakeDelegation(stake_delegation.0.clone()).into()
    }

    /// Create a Certificate for OwnerStakeDelegation
    pub fn owner_stake_delegation(owner_stake: &OwnerStakeDelegation) -> Certificate {
        certificate::Certificate::OwnerStakeDelegation(owner_stake.0.clone()).into()
    }

    /// Create a Certificate for PoolRegistration
    pub fn stake_pool_registration(pool_registration: &PoolRegistration) -> Certificate {
        certificate::Certificate::PoolRegistration(pool_registration.0.clone()).into()
    }

    /// Create a Certificate for PoolRetirement
    pub fn stake_pool_retirement(pool_retirement: &PoolRetirement) -> Certificate {
        certificate::Certificate::PoolRetirement(pool_retirement.0.clone()).into()
    }

    /// Create a Certificate for PoolUpdate
    pub fn stake_pool_update(pool_update: &PoolUpdate) -> Certificate {
        certificate::Certificate::PoolUpdate(pool_update.0.clone()).into()
    }

    pub fn get_type(&self) -> CertificateKind {
        match &self.0 {
            certificate::Certificate::StakeDelegation(_) => CertificateKind::StakeDelegation,
            certificate::Certificate::OwnerStakeDelegation(_) => {
                CertificateKind::OwnerStakeDelegation
            }
            certificate::Certificate::PoolRegistration(_) => CertificateKind::PoolRegistration,
            certificate::Certificate::PoolRetirement(_) => CertificateKind::PoolRetirement,
            certificate::Certificate::PoolUpdate(_) => CertificateKind::PoolUpdate,
        }
    }

    pub fn get_stake_delegation(&self) -> Result<StakeDelegation, JsValue> {
        match &self.0 {
            certificate::Certificate::StakeDelegation(cert) => Ok(cert.clone().into()),
            _ => Err(JsValue::from_str("Certificate is not StakeDelegation")),
        }
    }

    pub fn get_owner_stake_delegation(&self) -> Result<OwnerStakeDelegation, JsValue> {
        match &self.0 {
            certificate::Certificate::OwnerStakeDelegation(cert) => Ok(cert.clone().into()),
            _ => Err(JsValue::from_str("Certificate is not OwnerStakeDelegation")),
        }
    }

    pub fn get_pool_registration(&self) -> Result<PoolRegistration, JsValue> {
        match &self.0 {
            certificate::Certificate::PoolRegistration(cert) => Ok(cert.clone().into()),
            _ => Err(JsValue::from_str("Certificate is not PoolRegistration")),
        }
    }

    pub fn get_pool_retirement(&self) -> Result<PoolRetirement, JsValue> {
        match &self.0 {
            certificate::Certificate::PoolRetirement(cert) => Ok(cert.clone().into()),
            _ => Err(JsValue::from_str("Certificate is not PoolRetirement")),
        }
    }

    pub fn get_pool_update(&self) -> Result<PoolUpdate, JsValue> {
        match &self.0 {
            certificate::Certificate::PoolUpdate(cert) => Ok(cert.clone().into()),
            _ => Err(JsValue::from_str("Certificate is not PoolUpdate")),
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match &self.0 {
            certificate::Certificate::StakeDelegation(cert) => cert.serialize().as_ref().to_vec(),
            certificate::Certificate::OwnerStakeDelegation(cert) => cert.serialize().as_ref().to_vec(),
            certificate::Certificate::PoolRegistration(cert) => cert.serialize().as_ref().to_vec(),
            certificate::Certificate::PoolRetirement(cert) => cert.serialize().as_ref().to_vec(),
            certificate::Certificate::PoolUpdate(cert) => cert.serialize().as_ref().to_vec(),
        }
    }
}

#[wasm_bindgen]
impl PoolRegistration {
    #[wasm_bindgen(constructor)]
    pub fn new(
        serial: &U128,
        owners: &PublicKeys,
        operators: &PublicKeys,
        management_threshold: u8,
        start_validity: &TimeOffsetSeconds,
        leader_keys: &GenesisPraosLeader,
    ) -> PoolRegistration {
        use chain::certificate::PoolPermissions;
        chain::certificate::PoolRegistration {
            serial: serial.0.clone(),
            owners: owners.0.clone().into_iter().map(|key| key.0).collect(),
            operators: operators.0.clone().into_iter().map(|key| key.0).collect(),
            permissions: PoolPermissions::new(management_threshold),
            start_validity: start_validity.0.clone(),
            // TODO: Hardcoded parameter
            rewards: chain::rewards::TaxType::zero(),
            // TODO: Hardcoded parameter
            reward_account: None,
            keys: leader_keys.0.clone(),
        }
        .into()
    }

    pub fn id(&self) -> PoolId {
        self.0.to_id().into()
    }

    pub fn start_validity(&self) -> TimeOffsetSeconds {
        self.0.start_validity.into()
    }

    // TODO: missing PoolPermissions. Don't think we need this for now

    pub fn owners(&self) -> PublicKeys {
        PublicKeys(self.0.owners.iter().map(|key| key.clone().into()).collect())
    }

    pub fn operators(&self) -> PublicKeys {
        PublicKeys(self.0.operators.iter().map(|key| key.clone().into()).collect())
    }

    pub fn rewards(&self) -> TaxType {
        self.0.rewards.into()
    }

    pub fn reward_account(&self) -> Option<Account> {
        self.0.reward_account.as_ref().map(|acc| Account(acc.clone()))
    }

    pub fn keys(&self) -> GenesisPraosLeader {
        GenesisPraosLeader(self.0.keys.clone())
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.serialize().as_ref().to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<PoolRegistration, JsValue> {
        let mut buf = ReadBuf::from(&bytes);
        chain::certificate::PoolRegistration::read(&mut buf)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(PoolRegistration)
    }
}

#[wasm_bindgen]
pub struct TaxType(chain::rewards::TaxType);

impl From<chain::rewards::TaxType> for TaxType {
    fn from(inner: chain::rewards::TaxType) -> TaxType {
        TaxType(inner)
    }
}

impl TaxType {
    pub fn fixed(&self) -> Value {
        self.0.fixed.into()
    }

    pub fn ratio_numerator(&self) -> Value {
        Value::from(self.0.ratio.numerator)
    }

    pub fn ratio_denominator(&self) -> Value {
        Value::from(self.0.ratio.denominator.get())
    }

    pub fn max_limit(&self) -> Option<Value> {
        Some(Value::from(self.0.max_limit?.get()))
    }
}

#[wasm_bindgen]
pub struct TimeOffsetSeconds(chain_time::timeline::TimeOffsetSeconds);

impl From<chain_time::timeline::TimeOffsetSeconds> for TimeOffsetSeconds {
    fn from(inner: chain_time::timeline::TimeOffsetSeconds) -> TimeOffsetSeconds {
        TimeOffsetSeconds(inner)
    }
}

#[wasm_bindgen]
impl TimeOffsetSeconds {
    /// Parse the given string into a 64 bits unsigned number
    pub fn from_string(number: &str) -> Result<TimeOffsetSeconds, JsValue> {
        number
            .parse::<u64>()
            .map_err(|e| JsValue::from_str(&format! {"{:?}", e}))
            .map(chain_time::DurationSeconds)
            .map(|duration| chain_time::timeline::TimeOffsetSeconds::from(duration).into())
    }

    pub fn to_string(&self) -> String {
        format!("{}", u64::from(self.0))
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct PoolId(chain::certificate::PoolId);

impl From<chain::certificate::PoolId> for PoolId {
    fn from(pool_id: chain::certificate::PoolId) -> PoolId {
        PoolId(pool_id)
    }
}

#[wasm_bindgen]
impl PoolId {
    pub fn from_hex(hex_string: &str) -> Result<PoolId, JsValue> {
        crypto::Blake2b256::from_str(hex_string)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
            .map(|hash| PoolId(hash.into()))
    }

    pub fn to_string(&self) -> String {
        format!("{}", self.0).to_string()
    }
}

#[wasm_bindgen]
pub struct KesPublicKey(crypto::PublicKey<crypto::SumEd25519_12>);

impl From<crypto::PublicKey<crypto::SumEd25519_12>> for KesPublicKey {
    fn from(kes: crypto::PublicKey<crypto::SumEd25519_12>) -> KesPublicKey {
        KesPublicKey(kes)
    }
}

#[wasm_bindgen]
impl KesPublicKey {
    pub fn from_bech32(bech32_str: &str) -> Result<KesPublicKey, JsValue> {
        crypto::PublicKey::try_from_bech32_str(&bech32_str)
            .map(KesPublicKey)
            .map_err(|_| JsValue::from_str("Malformed kes public key"))
    }
}

#[wasm_bindgen]
pub struct VrfPublicKey(crypto::PublicKey<crypto::Curve25519_2HashDH>);

impl From<crypto::PublicKey<crypto::Curve25519_2HashDH>> for VrfPublicKey {
    fn from(vrf: crypto::PublicKey<crypto::Curve25519_2HashDH>) -> VrfPublicKey {
        VrfPublicKey(vrf)
    }
}

#[wasm_bindgen]
impl VrfPublicKey {
    pub fn from_bech32(bech32_str: &str) -> Result<VrfPublicKey, JsValue> {
        crypto::PublicKey::try_from_bech32_str(&bech32_str)
            .map(VrfPublicKey)
            .map_err(|_| JsValue::from_str("Malformed vrf public key"))
    }
}

/// Amount of the balance in the transaction.
#[wasm_bindgen]
pub struct Balance(tx::Balance);

impl From<tx::Balance> for Balance {
    fn from(balance: tx::Balance) -> Balance {
        Balance(balance)
    }
}

#[wasm_bindgen]
impl Balance {
    //Not sure is this is the best way
    pub fn get_sign(&self) -> JsValue {
        JsValue::from_str(match self.0 {
            tx::Balance::Positive(_) => "positive",
            tx::Balance::Negative(_) => "negative",
            tx::Balance::Zero => "zero",
        })
    }

    pub fn is_positive(&self) -> bool {
        match self.0 {
            tx::Balance::Positive(_) => true,
            _ => false,
        }
    }

    pub fn is_negative(&self) -> bool {
        match self.0 {
            tx::Balance::Negative(_) => true,
            _ => false,
        }
    }

    pub fn is_zero(&self) -> bool {
        match self.0 {
            tx::Balance::Zero => true,
            _ => false,
        }
    }

    /// Get value without taking into account if the balance is positive or negative
    pub fn get_value(&self) -> Value {
        match self.0 {
            tx::Balance::Positive(v) => Value(v),
            tx::Balance::Negative(v) => Value(v),
            tx::Balance::Zero => Value(value::Value(0)),
        }
    }
}

/// Algorithm used to compute transaction fees
/// Currently the only implementation is the Linear one
#[wasm_bindgen]
pub struct Fee(FeeVariant);

#[wasm_bindgen]
impl Fee {
    /// Linear algorithm, this is formed by: `coefficient * (#inputs + #outputs) + constant + certificate * #certificate
    pub fn linear_fee(constant: &Value, coefficient: &Value, certificate: &Value) -> Fee {
        Fee(FeeVariant::Linear(fee::LinearFee::new(
            *constant.0.as_ref(),
            *coefficient.0.as_ref(),
            *certificate.0.as_ref(),
        )))
    }

    pub fn calculate(&self, tx: &Transaction) -> Value {
        let fee_algorithm = match &self.0 {
            FeeVariant::Linear(algorithm) => algorithm,
        };

        use fee::FeeAlgorithm;
        let v = map_payloads!(&tx.0, tx, {
            fee_algorithm.calculate(
                tx.as_slice().payload().to_certificate_slice(),
                tx.nb_inputs(),
                tx.nb_outputs(),
            )
        });
        Value(v)
    }
}

pub enum FeeVariant {
    Linear(fee::LinearFee),
}

/// Structure that proofs that certain user agrees with
/// some data. This structure is used to sign `Transaction`
/// and get `SignedTransaction` out.
///
/// It's important that witness works with opaque structures
/// and may not know the contents of the internal transaction.
#[wasm_bindgen]
#[derive(Clone)]
pub struct Witness(tx::Witness);

impl From<tx::Witness> for Witness {
    fn from(witness: tx::Witness) -> Witness {
        Witness(witness)
    }
}

#[wasm_bindgen]
impl Witness {
    /// Generate Witness for an utxo-based transaction Input
    pub fn for_utxo(
        genesis_hash: &Hash,
        transaction_id: &TransactionSignDataHash,
        secret_key: &PrivateKey,
    ) -> Witness {
        Witness(tx::Witness::new_utxo(
            &genesis_hash.0,
            &transaction_id.0,
            &secret_key.0,
        ))
    }

    // Witness for a utxo-based transaction generated externally (such as hardware wallets)
    pub fn from_external_utxo(witness: &UtxoWitness) -> Witness {
        Witness(tx::Witness::Utxo(witness.0.clone()))
    }

    /// Generate Witness for an account based transaction Input
    /// the account-spending-counter should be incremented on each transaction from this account
    pub fn for_account(
        genesis_hash: &Hash,
        transaction_id: &TransactionSignDataHash,
        secret_key: &PrivateKey,
        account_spending_counter: &SpendingCounter,
    ) -> Witness {
        Witness(tx::Witness::new_account(
            &genesis_hash.0,
            &transaction_id.0,
            &account_spending_counter.0,
            &secret_key.0,
        ))
    }

    // Witness for a account-based transaction generated externally (such as hardware wallets)
    pub fn from_external_account(witness: &AccountWitness) -> Witness {
        Witness(tx::Witness::Account(witness.0.clone()))
    }

    /// Generate Witness for a legacy icarus utxo-based transaction Input
    pub fn for_legacy_icarus_utxo(
        genesis_hash: &Hash,
        transaction_id: &TransactionSignDataHash,
        secret_key: &Bip32PrivateKey,
    ) -> Witness {
        Witness(tx::Witness::new_old_icarus_utxo(
            &genesis_hash.0,
            &transaction_id.0,
            &secret_key.0,
        ))
    }

    // Witness for a legacy icarus utxo-based transaction generated externally (such as hardware wallets)
    pub fn from_external_legacy_icarus_utxo(
        key: &Bip32PublicKey,
        witness: &LegacyUtxoWitness,
    ) -> Witness {
        Witness(tx::Witness::OldUtxo(key.0.clone(), witness.0.clone()))
    }

    /// Generate Witness for a legacy daedalus utxo-based transaction Input
    pub fn for_legacy_daedalus_utxo(
        genesis_hash: &Hash,
        transaction_id: &TransactionSignDataHash,
        secret_key: &LegacyDaedalusPrivateKey,
    ) -> Witness {
        Witness(tx::Witness::new_old_daedalus_utxo(
            &genesis_hash.0,
            &transaction_id.0,
            &secret_key.0,
        ))
    }

    /// Get string representation
    pub fn to_bech32(&self) -> Result<String, JsValue> {
        let bytes = self
            .0
            .serialize_as_vec()
            .map_err(|error| JsValue::from_str(&format!("{}", error)))?;

        Bech32::new("witness".to_string(), bytes.to_base32())
            .map(|bech32| bech32.to_string())
            .map_err(|error| JsValue::from_str(&format!("{}", error)))
    }
}

impl_collection!(Witnesses, Witness);

#[wasm_bindgen]
pub struct SpendingCounter(account::SpendingCounter);

impl From<account::SpendingCounter> for SpendingCounter {
    fn from(spending_counter: account::SpendingCounter) -> SpendingCounter {
        SpendingCounter(spending_counter)
    }
}

/// Spending counter associated to an account.
///
/// every time the owner is spending from an account,
/// the counter is incremented. A matching counter
/// needs to be used in the spending phase to make
/// sure we have non-replayability of a transaction.
#[wasm_bindgen]
impl SpendingCounter {
    pub fn zero() -> Self {
        account::SpendingCounter::zero().into()
    }

    pub fn from_u32(counter: u32) -> Self {
        account::SpendingCounter::from(counter).into()
    }
}

#[wasm_bindgen]
pub struct OldUtxoDeclaration(chain::legacy::UtxoDeclaration);

#[wasm_bindgen]
impl OldUtxoDeclaration {
    pub fn size(&self) -> usize {
        self.0.addrs.len()
    }

    pub fn get_address(&self, index: usize) -> String {
        format!("{}", self.0.addrs[index].0)
    }

    pub fn get_value(&self, index: usize) -> Value {
        self.0.addrs[index].1.into()
    }
}

/// All possible messages recordable in the Block content
#[wasm_bindgen]
#[derive(Clone)]
pub struct Fragment(chain::fragment::Fragment);

impl From<chain::fragment::Fragment> for Fragment {
    fn from(msg: chain::fragment::Fragment) -> Fragment {
        Fragment(msg)
    }
}

#[wasm_bindgen]
impl Fragment {
    pub fn from_transaction(tx: &Transaction) -> Fragment {
        use chain::fragment::Fragment as F;
        use TaggedTransaction as T;
        match tx.0.clone() {
            T::NoExtra(auth_tx) => F::Transaction(auth_tx),
            T::PoolRegistration(auth_tx) => F::PoolRegistration(auth_tx),
            T::PoolRetirement(auth_tx) => F::PoolRetirement(auth_tx),
            T::PoolUpdate(auth_tx) => F::PoolUpdate(auth_tx),
            T::StakeDelegation(auth_tx) => F::StakeDelegation(auth_tx),
            T::OwnerStakeDelegation(auth_tx) => F::OwnerStakeDelegation(auth_tx),
        }
        .into()
    }

    /// Get a Transaction if the Fragment represents one
    pub fn get_transaction(&self) -> Result<Transaction, JsValue> {
        use chain::fragment::Fragment as F;
        use TaggedTransaction as T;
        match self.0.clone() {
            F::Transaction(auth) => Ok(T::NoExtra(auth)),
            F::OwnerStakeDelegation(auth) => Ok(T::OwnerStakeDelegation(auth)),
            F::StakeDelegation(auth) => Ok(T::StakeDelegation(auth)),
            F::PoolRegistration(auth) => Ok(T::PoolRegistration(auth)),
            F::PoolRetirement(auth) => Ok(T::PoolRetirement(auth)),
            F::PoolUpdate(auth) => Ok(T::PoolUpdate(auth)),
            _ => Err(JsValue::from_str("Invalid fragment type")),
        }
        .map(Transaction)
    }

    pub fn get_old_utxo_declaration(&self) -> Result<OldUtxoDeclaration, JsValue> {
        match self.0.clone() {
            chain::fragment::Fragment::OldUtxoDeclaration(decl) => Ok(OldUtxoDeclaration(decl)),
            _ => Err(JsValue::from_str("fragment is not OldUtxoDeclaration")),
        }
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, JsValue> {
        self.0
            .serialize_as_vec()
            .map_err(|error| JsValue::from_str(&format!("{}", error)))
    }

    pub fn from_bytes(bytes: Uint8Array) -> Result<Fragment, JsValue> {
        let mut slice: Box<[u8]> = vec![0; bytes.length() as usize].into_boxed_slice();
        bytes.copy_to(&mut *slice);
        chain::fragment::Fragment::deserialize(&*slice)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(Fragment)
    }

    pub fn is_initial(&self) -> bool {
        match self.0 {
            chain::fragment::Fragment::Initial(_) => true,
            _ => false,
        }
    }

    pub fn is_transaction(&self) -> bool {
        match self.0 {
            chain::fragment::Fragment::Transaction(_) => true,
            _ => false,
        }
    }

    pub fn is_owner_stake_delegation(&self) -> bool {
        match self.0 {
            chain::fragment::Fragment::OwnerStakeDelegation(_) => true,
            _ => false,
        }
    }

    pub fn is_stake_delegation(&self) -> bool {
        match self.0 {
            chain::fragment::Fragment::StakeDelegation(_) => true,
            _ => false,
        }
    }

    pub fn is_pool_registration(&self) -> bool {
        match self.0 {
            chain::fragment::Fragment::PoolRegistration(_) => true,
            _ => false,
        }
    }

    pub fn is_pool_retirement(&self) -> bool {
        match self.0 {
            chain::fragment::Fragment::PoolRetirement(_) => true,
            _ => false,
        }
    }

    pub fn is_pool_update(&self) -> bool {
        match self.0 {
            chain::fragment::Fragment::PoolUpdate(_) => true,
            _ => false,
        }
    }

    pub fn is_old_utxo_declaration(&self) -> bool {
        match self.0 {
            chain::fragment::Fragment::OldUtxoDeclaration(_) => true,
            _ => false,
        }
    }

    pub fn is_update_proposal(&self) -> bool {
        match self.0 {
            chain::fragment::Fragment::UpdateProposal(_) => true,
            _ => false,
        }
    }

    pub fn is_update_vote(&self) -> bool {
        match self.0 {
            chain::fragment::Fragment::UpdateVote(_) => true,
            _ => false,
        }
    }

    pub fn id(&self) -> FragmentId {
        self.0.id().into()
    }
}

/// `Block` is an element of the blockchain it contains multiple
/// transaction and a reference to the parent block. Alongside
/// with the position of that block in the chain.
#[wasm_bindgen]
pub struct Block(chain::block::Block);

impl From<chain::block::Block> for Block {
    fn from(block: chain::block::Block) -> Block {
        Block(block)
    }
}

#[wasm_bindgen]
impl Block {
    /// Deserialize a block from a byte array
    pub fn from_bytes(bytes: Uint8Array) -> Result<Block, JsValue> {
        let mut slice: Box<[u8]> = vec![0; bytes.length() as usize].into_boxed_slice();
        bytes.copy_to(&mut *slice);
        chain::block::Block::deserialize(&*slice)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(Block)
    }

    pub fn id(&self) -> BlockId {
        self.0.id().into()
    }

    pub fn parent_id(&self) -> BlockId {
        self.0.parent_id().into()
    }

    ///This involves copying all the fragments
    pub fn fragments(&self) -> Fragments {
        self.0
            .fragments()
            .map(|m| Fragment::from(m.clone()))
            .collect::<Vec<Fragment>>()
            .into()
    }

    pub fn epoch(&self) -> u32 {
        self.0.date().epoch
    }

    pub fn slot(&self) -> u32 {
        self.0.date().slot_id
    }

    pub fn chain_length(&self) -> u32 {
        u32::from(self.0.chain_length())
    }

    pub fn leader_id(&self) -> Option<PoolId> {
        Some(self.0.header.get_stakepool_id()?.into())
    }

    pub fn content_size(&self) -> u32 {
        self.0.header.block_content_size()
    }
}

#[wasm_bindgen]
pub struct BlockId(key::Hash);

impl From<key::Hash> for BlockId {
    fn from(block_id: key::Hash) -> BlockId {
        BlockId(block_id)
    }
}

#[wasm_bindgen]
impl BlockId {
    pub fn calculate(bytes: &[u8]) -> Hash {
        key::Hash::hash_bytes(bytes).into()
    }

    pub fn from_bytes(bytes: Uint8Array) -> Result<BlockId, JsValue> {
        let mut slice: Box<[u8]> = vec![0; bytes.length() as usize].into_boxed_slice();
        bytes.copy_to(&mut *slice);
        key::Hash::deserialize(&*slice)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(BlockId)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.serialize_as_vec().unwrap()
    }
}

#[wasm_bindgen]
pub struct FragmentId(chain::fragment::FragmentId);

impl From<chain::fragment::FragmentId> for FragmentId {
    fn from(fragment_id: chain::fragment::FragmentId) -> FragmentId {
        FragmentId(fragment_id)
    }
}

#[wasm_bindgen]
impl FragmentId {
    pub fn calculate(bytes: &[u8]) -> FragmentId {
        key::Hash::hash_bytes(bytes).into()
    }

    pub fn from_bytes(bytes: Uint8Array) -> Result<FragmentId, JsValue> {
        let mut slice: Box<[u8]> = vec![0; bytes.length() as usize].into_boxed_slice();
        bytes.copy_to(&mut *slice);
        chain::fragment::FragmentId::deserialize(&*slice)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
            .map(FragmentId)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.serialize_as_vec().unwrap()
    }
}

//this is useful for debugging, I'm not sure it is a good idea to have it here

#[wasm_bindgen]
pub fn uint8array_to_hex(input: JsValue) -> Result<String, JsValue> {
    //For some reason JSON.stringify serializes Uint8Array as objects instead of arrays
    let input_array: std::collections::BTreeMap<usize, u8> = input
        .into_serde()
        .map_err(|e| JsValue::from_str(&format!("{}", e)))?;

    let mut s = String::with_capacity(input_array.len() * 2);

    for &byte in input_array.values() {
        s.push_str(&hex::encode([byte]));
    }

    Ok(s)
}

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
