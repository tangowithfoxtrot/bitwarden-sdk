use bitwarden_api_api::models::CipherDetailsResponseModel;
use bitwarden_crypto::{
    CryptoError, DecryptedString, DecryptedVec, EncString, KeyContainer, KeyDecryptable,
    KeyEncryptable, LocateKey, SensitiveString, SymmetricCryptoKey,
};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use uuid::Uuid;

use super::{
    attachment, card, field, identity,
    local_data::{LocalData, LocalDataView},
    login, secure_note,
};
use crate::{
    error::{require, Error, Result},
    vault::password_history,
};

#[derive(Clone, Copy, Serialize_repr, Deserialize_repr, Debug, JsonSchema)]
#[repr(u8)]
#[cfg_attr(feature = "mobile", derive(uniffi::Enum))]
pub enum CipherType {
    Login = 1,
    SecureNote = 2,
    Card = 3,
    Identity = 4,
}

#[derive(Clone, Copy, Serialize_repr, Deserialize_repr, Debug, JsonSchema)]
#[repr(u8)]
#[cfg_attr(feature = "mobile", derive(uniffi::Enum))]
pub enum CipherRepromptType {
    None = 0,
    Password = 1,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "mobile", derive(uniffi::Record))]
pub struct Cipher {
    pub id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub collection_ids: Vec<Uuid>,

    /// More recent ciphers uses individual encryption keys to encrypt the other fields of the
    /// Cipher.
    pub key: Option<EncString>,

    pub name: EncString,
    pub notes: Option<EncString>,

    pub r#type: CipherType,
    pub login: Option<login::Login>,
    pub identity: Option<identity::Identity>,
    pub card: Option<card::Card>,
    pub secure_note: Option<secure_note::SecureNote>,

    pub favorite: bool,
    pub reprompt: CipherRepromptType,
    pub organization_use_totp: bool,
    pub edit: bool,
    pub view_password: bool,
    pub local_data: Option<LocalData>,

    pub attachments: Option<Vec<attachment::Attachment>>,
    pub fields: Option<Vec<field::Field>>,
    pub password_history: Option<Vec<password_history::PasswordHistory>>,

    pub creation_date: DateTime<Utc>,
    pub deleted_date: Option<DateTime<Utc>>,
    pub revision_date: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "mobile", derive(uniffi::Record))]
pub struct CipherView {
    pub id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub collection_ids: Vec<Uuid>,

    pub key: Option<EncString>,

    pub name: DecryptedString,
    pub notes: Option<DecryptedString>,

    pub r#type: CipherType,
    pub login: Option<login::LoginView>,
    pub identity: Option<identity::IdentityView>,
    pub card: Option<card::CardView>,
    pub secure_note: Option<secure_note::SecureNoteView>,

    pub favorite: bool,
    pub reprompt: CipherRepromptType,
    pub organization_use_totp: bool,
    pub edit: bool,
    pub view_password: bool,
    pub local_data: Option<LocalDataView>,

    pub attachments: Option<Vec<attachment::AttachmentView>>,
    pub fields: Option<Vec<field::FieldView>>,
    pub password_history: Option<Vec<password_history::PasswordHistoryView>>,

    pub creation_date: DateTime<Utc>,
    pub deleted_date: Option<DateTime<Utc>>,
    pub revision_date: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "mobile", derive(uniffi::Record))]
pub struct CipherListView {
    pub id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub collection_ids: Vec<Uuid>,

    pub name: DecryptedString,
    pub sub_title: DecryptedString,

    pub r#type: CipherType,

    pub favorite: bool,
    pub reprompt: CipherRepromptType,
    pub edit: bool,
    pub view_password: bool,

    /// The number of attachments
    pub attachments: u32,

    pub creation_date: DateTime<Utc>,
    pub deleted_date: Option<DateTime<Utc>>,
    pub revision_date: DateTime<Utc>,
}

impl KeyEncryptable<SymmetricCryptoKey, Cipher> for CipherView {
    fn encrypt_with_key(mut self, key: &SymmetricCryptoKey) -> Result<Cipher, CryptoError> {
        let ciphers_key = Cipher::get_cipher_key(key, &self.key)?;
        let key = ciphers_key.as_ref().unwrap_or(key);

        // For compatibility reasons, we only create checksums for ciphers that have a key
        if ciphers_key.is_some() {
            self.generate_checksums();
        }

        Ok(Cipher {
            id: self.id,
            organization_id: self.organization_id,
            folder_id: self.folder_id,
            collection_ids: self.collection_ids,
            key: self.key,
            name: self.name.encrypt_with_key(key)?,
            notes: self.notes.encrypt_with_key(key)?,
            r#type: self.r#type,
            login: self.login.encrypt_with_key(key)?,
            identity: self.identity.encrypt_with_key(key)?,
            card: self.card.encrypt_with_key(key)?,
            secure_note: self.secure_note.encrypt_with_key(key)?,
            favorite: self.favorite,
            reprompt: self.reprompt,
            organization_use_totp: self.organization_use_totp,
            edit: self.edit,
            view_password: self.view_password,
            local_data: self.local_data.encrypt_with_key(key)?,
            attachments: self.attachments.encrypt_with_key(key)?,
            fields: self.fields.encrypt_with_key(key)?,
            password_history: self.password_history.encrypt_with_key(key)?,
            creation_date: self.creation_date,
            deleted_date: self.deleted_date,
            revision_date: self.revision_date,
        })
    }
}

impl KeyDecryptable<SymmetricCryptoKey, CipherView> for Cipher {
    fn decrypt_with_key(&self, key: &SymmetricCryptoKey) -> Result<CipherView, CryptoError> {
        let ciphers_key = Cipher::get_cipher_key(key, &self.key)?;
        let key = ciphers_key.as_ref().unwrap_or(key);

        let mut cipher = CipherView {
            id: self.id,
            organization_id: self.organization_id,
            folder_id: self.folder_id,
            collection_ids: self.collection_ids.clone(),
            key: self.key.clone(),
            name: self.name.decrypt_with_key(key).ok().unwrap_or_default(),
            notes: self.notes.decrypt_with_key(key).ok().flatten(),
            r#type: self.r#type,
            login: self.login.decrypt_with_key(key).ok().flatten(),
            identity: self.identity.decrypt_with_key(key).ok().flatten(),
            card: self.card.decrypt_with_key(key).ok().flatten(),
            secure_note: self.secure_note.decrypt_with_key(key).ok().flatten(),
            favorite: self.favorite,
            reprompt: self.reprompt,
            organization_use_totp: self.organization_use_totp,
            edit: self.edit,
            view_password: self.view_password,
            local_data: self.local_data.decrypt_with_key(key).ok().flatten(),
            attachments: self.attachments.decrypt_with_key(key).ok().flatten(),
            fields: self.fields.decrypt_with_key(key).ok().flatten(),
            password_history: self.password_history.decrypt_with_key(key).ok().flatten(),
            creation_date: self.creation_date,
            deleted_date: self.deleted_date,
            revision_date: self.revision_date,
        };

        // For compatibility we only remove URLs with invalid checksums if the cipher has a key
        if ciphers_key.is_some() {
            cipher.remove_invalid_checksums();
        }

        Ok(cipher)
    }
}

impl Cipher {
    /// Get the decrypted individual encryption key for this cipher.
    /// Note that some ciphers do not have individual encryption keys,
    /// in which case this will return Ok(None) and the key associated
    /// with this cipher's user or organization must be used instead
    pub(super) fn get_cipher_key(
        key: &SymmetricCryptoKey,
        ciphers_key: &Option<EncString>,
    ) -> Result<Option<SymmetricCryptoKey>, CryptoError> {
        ciphers_key
            .as_ref()
            .map(|k| {
                let key: DecryptedVec = k.decrypt_with_key(key)?;
                SymmetricCryptoKey::try_from(key)
            })
            .transpose()
    }

    fn get_decrypted_subtitle(
        &self,
        key: &SymmetricCryptoKey,
    ) -> Result<DecryptedString, CryptoError> {
        Ok(match self.r#type {
            CipherType::Login => {
                let Some(login) = &self.login else {
                    return Ok(SensitiveString::default());
                };
                login.username.decrypt_with_key(key)?.unwrap_or_default()
            }
            CipherType::SecureNote => SensitiveString::default(),
            CipherType::Card => {
                let Some(card) = &self.card else {
                    return Ok(SensitiveString::default());
                };

                build_subtitle_card(
                    card.brand
                        .as_ref()
                        .map(|b| b.decrypt_with_key(key))
                        .transpose()?,
                    card.number
                        .as_ref()
                        .map(|n| n.decrypt_with_key(key))
                        .transpose()?,
                )
            }
            CipherType::Identity => {
                let Some(identity) = &self.identity else {
                    return Ok(SensitiveString::default());
                };

                build_subtitle_identity(
                    identity
                        .first_name
                        .as_ref()
                        .map(|f| f.decrypt_with_key(key))
                        .transpose()?,
                    identity
                        .last_name
                        .as_ref()
                        .map(|l| l.decrypt_with_key(key))
                        .transpose()?,
                )
            }
        })
    }
}

/// Builds the subtitle for a card cipher
///
/// Care is taken to avoid leaking sensitive data by allocating the full size of the subtitle
fn build_subtitle_card(
    brand: Option<DecryptedString>,
    number: Option<DecryptedString>,
) -> SensitiveString {
    let brand: Option<SensitiveString> = brand.filter(|b: &SensitiveString| !b.expose().is_empty());

    // We only want to expose the last 4 or 5 digits of the card number
    let number: Option<SensitiveString> = number
        .filter(|b: &SensitiveString| b.expose().len() > 4)
        .map(|n| {
            // For AMEX cards show 5 digits instead of 4
            let desired_len = match &n.expose()[0..2] {
                "34" | "37" => 5,
                _ => 4,
            };
            let start = n.expose().len() - desired_len;

            let mut str = SensitiveString::new(Box::new(String::with_capacity(desired_len + 1)));
            str.expose_mut().push('*');
            str.expose_mut().push_str(&n.expose()[start..]);

            str
        });

    match (brand, number) {
        (Some(brand), Some(number)) => {
            let length = brand.expose().len() + 2 + number.expose().len();

            let mut str = SensitiveString::new(Box::new(String::with_capacity(length)));
            str.expose_mut().push_str(brand.expose());
            str.expose_mut().push_str(", ");
            str.expose_mut().push_str(number.expose());

            str
        }
        (Some(brand), None) => brand,
        (None, Some(number)) => number,
        _ => SensitiveString::new(Box::new("".to_owned())),
    }
}

/// Builds the subtitle for a card cipher
///
/// Care is taken to avoid leaking sensitive data by allocating the full size of the subtitle
fn build_subtitle_identity(
    first_name: Option<DecryptedString>,
    last_name: Option<DecryptedString>,
) -> SensitiveString {
    let first_name: Option<SensitiveString> =
        first_name.filter(|f: &SensitiveString| !f.expose().is_empty());
    let last_name: Option<SensitiveString> =
        last_name.filter(|l: &SensitiveString| !l.expose().is_empty());

    match (first_name, last_name) {
        (Some(first_name), Some(last_name)) => {
            let length = first_name.expose().len() + 1 + last_name.expose().len();

            let mut str = SensitiveString::new(Box::new(String::with_capacity(length)));
            str.expose_mut().push_str(first_name.expose());
            str.expose_mut().push(' ');
            str.expose_mut().push_str(last_name.expose());

            str
        }
        (Some(first_name), None) => first_name,
        (None, Some(last_name)) => last_name,
        _ => SensitiveString::new(Box::new("".to_owned())),
    }
}

impl CipherView {
    pub fn generate_cipher_key(&mut self, key: &SymmetricCryptoKey) -> Result<()> {
        let ciphers_key = Cipher::get_cipher_key(key, &self.key)?;
        let key = ciphers_key.as_ref().unwrap_or(key);

        let new_key = SymmetricCryptoKey::generate(rand::thread_rng());

        self.key = Some(new_key.to_vec().expose().encrypt_with_key(key)?);
        Ok(())
    }

    pub fn generate_checksums(&mut self) {
        if let Some(uris) = self.login.as_mut().and_then(|l| l.uris.as_mut()) {
            for uri in uris {
                uri.generate_checksum();
            }
        }
    }

    pub fn remove_invalid_checksums(&mut self) {
        if let Some(uris) = self.login.as_mut().and_then(|l| l.uris.as_mut()) {
            uris.retain(|u| u.is_checksum_valid());
        }
    }

    pub fn move_to_organization(
        &mut self,
        enc: &dyn KeyContainer,
        organization_id: Uuid,
    ) -> Result<()> {
        // If the cipher has a key, we need to re-encrypt it with the new organization key
        if let Some(cipher_key) = &mut self.key {
            let old_key = enc
                .get_key(&self.organization_id)
                .ok_or(Error::VaultLocked)?;

            let new_key = enc
                .get_key(&Some(organization_id))
                .ok_or(Error::VaultLocked)?;

            let dec_cipher_key: DecryptedVec = cipher_key.decrypt_with_key(old_key)?;
            *cipher_key = dec_cipher_key.expose().encrypt_with_key(new_key)?;
        }

        self.organization_id = Some(organization_id);
        Ok(())
    }
}

impl KeyDecryptable<SymmetricCryptoKey, CipherListView> for Cipher {
    fn decrypt_with_key(&self, key: &SymmetricCryptoKey) -> Result<CipherListView, CryptoError> {
        let ciphers_key = Cipher::get_cipher_key(key, &self.key)?;
        let key = ciphers_key.as_ref().unwrap_or(key);

        Ok(CipherListView {
            id: self.id,
            organization_id: self.organization_id,
            folder_id: self.folder_id,
            collection_ids: self.collection_ids.clone(),
            name: self.name.decrypt_with_key(key).ok().unwrap_or_default(),
            sub_title: self.get_decrypted_subtitle(key).ok().unwrap_or_default(),
            r#type: self.r#type,
            favorite: self.favorite,
            reprompt: self.reprompt,
            edit: self.edit,
            view_password: self.view_password,
            attachments: self
                .attachments
                .as_ref()
                .map(|a| a.len() as u32)
                .unwrap_or(0),
            creation_date: self.creation_date,
            deleted_date: self.deleted_date,
            revision_date: self.revision_date,
        })
    }
}

impl LocateKey for Cipher {
    fn locate_key<'a>(
        &self,
        enc: &'a dyn KeyContainer,
        _: &Option<Uuid>,
    ) -> Option<&'a SymmetricCryptoKey> {
        enc.get_key(&self.organization_id)
    }
}
impl LocateKey for CipherView {
    fn locate_key<'a>(
        &self,
        enc: &'a dyn KeyContainer,
        _: &Option<Uuid>,
    ) -> Option<&'a SymmetricCryptoKey> {
        enc.get_key(&self.organization_id)
    }
}

impl TryFrom<CipherDetailsResponseModel> for Cipher {
    type Error = Error;

    fn try_from(cipher: CipherDetailsResponseModel) -> Result<Self> {
        Ok(Self {
            id: cipher.id,
            organization_id: cipher.organization_id,
            folder_id: cipher.folder_id,
            collection_ids: cipher.collection_ids.unwrap_or_default(),
            name: require!(EncString::try_from_optional(cipher.name)?),
            notes: EncString::try_from_optional(cipher.notes)?,
            r#type: require!(cipher.r#type).into(),
            login: cipher.login.map(|l| (*l).try_into()).transpose()?,
            identity: cipher.identity.map(|i| (*i).try_into()).transpose()?,
            card: cipher.card.map(|c| (*c).try_into()).transpose()?,
            secure_note: cipher.secure_note.map(|s| (*s).try_into()).transpose()?,
            favorite: cipher.favorite.unwrap_or(false),
            reprompt: cipher
                .reprompt
                .map(|r| r.into())
                .unwrap_or(CipherRepromptType::None),
            organization_use_totp: cipher.organization_use_totp.unwrap_or(true),
            edit: cipher.edit.unwrap_or(true),
            view_password: cipher.view_password.unwrap_or(true),
            local_data: None, // Not sent from server
            attachments: cipher
                .attachments
                .map(|a| a.into_iter().map(|a| a.try_into()).collect())
                .transpose()?,
            fields: cipher
                .fields
                .map(|f| f.into_iter().map(|f| f.try_into()).collect())
                .transpose()?,
            password_history: cipher
                .password_history
                .map(|p| p.into_iter().map(|p| p.try_into()).collect())
                .transpose()?,
            creation_date: require!(cipher.creation_date).parse()?,
            deleted_date: cipher.deleted_date.map(|d| d.parse()).transpose()?,
            revision_date: require!(cipher.revision_date).parse()?,
            key: EncString::try_from_optional(cipher.key)?,
        })
    }
}

impl From<bitwarden_api_api::models::CipherType> for CipherType {
    fn from(t: bitwarden_api_api::models::CipherType) -> Self {
        match t {
            bitwarden_api_api::models::CipherType::Login => CipherType::Login,
            bitwarden_api_api::models::CipherType::SecureNote => CipherType::SecureNote,
            bitwarden_api_api::models::CipherType::Card => CipherType::Card,
            bitwarden_api_api::models::CipherType::Identity => CipherType::Identity,
        }
    }
}

impl From<bitwarden_api_api::models::CipherRepromptType> for CipherRepromptType {
    fn from(t: bitwarden_api_api::models::CipherRepromptType) -> Self {
        match t {
            bitwarden_api_api::models::CipherRepromptType::None => CipherRepromptType::None,
            bitwarden_api_api::models::CipherRepromptType::Password => CipherRepromptType::Password,
        }
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use super::*;

    fn generate_cipher() -> CipherView {
        CipherView {
            r#type: CipherType::Login,
            login: Some(login::LoginView {
                username: Some(DecryptedString::test("test_username")),
                password: Some(DecryptedString::test("test_password")),
                password_revision_date: None,
                uris: None,
                totp: None,
                autofill_on_page_load: None,
                fido2_credentials: None,
            }),
            id: "fd411a1a-fec8-4070-985d-0e6560860e69".parse().ok(),
            organization_id: None,
            folder_id: None,
            collection_ids: vec![],
            key: None,
            name: DecryptedString::test("My test login"),
            notes: None,
            identity: None,
            card: None,
            secure_note: None,
            favorite: false,
            reprompt: CipherRepromptType::None,
            organization_use_totp: true,
            edit: true,
            view_password: true,
            local_data: None,
            attachments: None,
            fields: None,
            password_history: None,
            creation_date: "2024-01-30T17:55:36.150Z".parse().unwrap(),
            deleted_date: None,
            revision_date: "2024-01-30T17:55:36.150Z".parse().unwrap(),
        }
    }

    #[test]
    fn test_generate_cipher_key() {
        let key = SymmetricCryptoKey::generate(rand::thread_rng());

        let original_cipher = generate_cipher();

        // Check that the cipher gets encrypted correctly without it's own key
        let cipher = generate_cipher();
        let no_key_cipher_enc = cipher.encrypt_with_key(&key).unwrap();
        let no_key_cipher_dec: CipherView = no_key_cipher_enc.decrypt_with_key(&key).unwrap();
        assert!(no_key_cipher_dec.key.is_none());
        assert_eq!(no_key_cipher_dec.name, original_cipher.name);

        let mut cipher = generate_cipher();
        cipher.generate_cipher_key(&key).unwrap();

        // Check that the cipher gets encrypted correctly when it's assigned it's own key
        let key_cipher_enc = cipher.encrypt_with_key(&key).unwrap();
        let key_cipher_dec: CipherView = key_cipher_enc.decrypt_with_key(&key).unwrap();
        assert!(key_cipher_dec.key.is_some());
        assert_eq!(key_cipher_dec.name, original_cipher.name);
    }

    struct MockKeyContainer(HashMap<Option<Uuid>, SymmetricCryptoKey>);
    impl KeyContainer for MockKeyContainer {
        fn get_key<'a>(&'a self, org_id: &Option<Uuid>) -> Option<&'a SymmetricCryptoKey> {
            self.0.get(org_id)
        }
    }

    #[test]
    fn test_move_user_cipher_to_org() {
        let org = uuid::Uuid::new_v4();

        let enc = MockKeyContainer(HashMap::from([
            (None, SymmetricCryptoKey::generate(rand::thread_rng())),
            (Some(org), SymmetricCryptoKey::generate(rand::thread_rng())),
        ]));

        // Create a cipher with a user key
        let mut cipher = generate_cipher();
        cipher
            .generate_cipher_key(enc.get_key(&None).unwrap())
            .unwrap();

        cipher.move_to_organization(&enc, org).unwrap();
        assert_eq!(cipher.organization_id, Some(org));

        // Check that the cipher can be encrypted/decrypted with the new org key
        let org_key = enc.get_key(&Some(org)).unwrap();
        let cipher_enc = cipher.encrypt_with_key(org_key).unwrap();
        let cipher_dec: CipherView = cipher_enc.decrypt_with_key(org_key).unwrap();

        assert_eq!(cipher_dec.name.expose(), "My test login");
    }

    #[test]
    fn test_move_user_cipher_to_org_manually() {
        let org = uuid::Uuid::new_v4();

        let enc = MockKeyContainer(HashMap::from([
            (None, SymmetricCryptoKey::generate(rand::thread_rng())),
            (Some(org), SymmetricCryptoKey::generate(rand::thread_rng())),
        ]));

        // Create a cipher with a user key
        let mut cipher = generate_cipher();
        cipher
            .generate_cipher_key(enc.get_key(&None).unwrap())
            .unwrap();

        cipher.organization_id = Some(org);

        // Check that the cipher can not be encrypted, as the
        // cipher key is tied to the user key and not the org key
        let org_key = enc.get_key(&Some(org)).unwrap();
        assert!(cipher.encrypt_with_key(org_key).is_err());
    }

    #[test]
    fn test_build_subtitle_card_visa() {
        let brand = Some(DecryptedString::test("Visa"));
        let number = Some(DecryptedString::test("4111111111111111"));

        let subtitle = build_subtitle_card(brand, number);
        assert_eq!(subtitle.expose(), "Visa, *1111");
    }

    #[test]
    fn test_build_subtitle_card_mastercard() {
        let brand = Some(DecryptedString::test("Mastercard"));
        let number = Some(DecryptedString::test("5555555555554444"));

        let subtitle = build_subtitle_card(brand, number);
        assert_eq!(subtitle.expose(), "Mastercard, *4444");
    }

    #[test]
    fn test_build_subtitle_card_amex() {
        let brand = Some(DecryptedString::test("Amex"));
        let number = Some(DecryptedString::test("378282246310005"));

        let subtitle = build_subtitle_card(brand, number);
        assert_eq!(subtitle.expose(), "Amex, *10005");
    }

    #[test]
    fn test_build_subtitle_card_underflow() {
        let brand = Some(DecryptedString::test("Mastercard"));
        let number = Some(DecryptedString::test("4"));

        let subtitle = build_subtitle_card(brand, number);
        assert_eq!(subtitle.expose(), "Mastercard");
    }

    #[test]
    fn test_build_subtitle_card_only_brand() {
        let brand = Some(DecryptedString::test("Mastercard"));
        let number = None;

        let subtitle = build_subtitle_card(brand, number);
        assert_eq!(subtitle.expose(), "Mastercard");
    }

    #[test]
    fn test_build_subtitle_card_only_card() {
        let brand = None;
        let number = Some(DecryptedString::test("5555555555554444"));

        let subtitle = build_subtitle_card(brand, number);
        assert_eq!(subtitle.expose(), "*4444");
    }

    #[test]
    fn test_build_subtitle_identity() {
        let first_name = Some(DecryptedString::test("John"));
        let last_name = Some(DecryptedString::test("Doe"));

        let subtitle = build_subtitle_identity(first_name, last_name);
        assert_eq!(subtitle.expose(), "John Doe");
    }

    #[test]
    fn test_build_subtitle_identity_only_first() {
        let first_name = Some(DecryptedString::test("John"));
        let last_name = None;

        let subtitle = build_subtitle_identity(first_name, last_name);
        assert_eq!(subtitle.expose(), "John");
    }

    #[test]
    fn test_build_subtitle_identity_only_last() {
        let first_name = None;
        let last_name = Some(DecryptedString::test("Doe"));

        let subtitle = build_subtitle_identity(first_name, last_name);
        assert_eq!(subtitle.expose(), "Doe");
    }

    #[test]
    fn test_build_subtitle_identity_none() {
        let first_name = None;
        let last_name = None;

        let subtitle = build_subtitle_identity(first_name, last_name);
        assert_eq!(subtitle.expose(), "");
    }
}
