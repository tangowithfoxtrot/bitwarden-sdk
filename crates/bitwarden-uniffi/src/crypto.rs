use std::sync::Arc;

use bitwarden::mobile::crypto::{
    DerivePinKeyResponse, InitOrgCryptoRequest, InitUserCryptoRequest, UpdatePasswordResponse,
};
use bitwarden_crypto::{AsymmetricEncString, EncString, SensitiveString};

use crate::{error::Result, Client};

#[derive(uniffi::Object)]
pub struct ClientCrypto(pub(crate) Arc<Client>);

#[uniffi::export(async_runtime = "tokio")]
impl ClientCrypto {
    /// Initialization method for the user crypto. Needs to be called before any other crypto
    /// operations.
    pub async fn initialize_user_crypto(&self, req: InitUserCryptoRequest) -> Result<()> {
        Ok(self
            .0
             .0
            .write()
            .await
            .crypto()
            .initialize_user_crypto(req)
            .await?)
    }

    /// Initialization method for the organization crypto. Needs to be called after
    /// `initialize_user_crypto` but before any other crypto operations.
    pub async fn initialize_org_crypto(&self, req: InitOrgCryptoRequest) -> Result<()> {
        Ok(self
            .0
             .0
            .write()
            .await
            .crypto()
            .initialize_org_crypto(req)
            .await?)
    }

    /// Get the uses's decrypted encryption key. Note: It's very important
    /// to keep this key safe, as it can be used to decrypt all of the user's data
    pub async fn get_user_encryption_key(&self) -> Result<SensitiveString> {
        Ok(self
            .0
             .0
            .write()
            .await
            .crypto()
            .get_user_encryption_key()
            .await?)
    }

    /// Update the user's password, which will re-encrypt the user's encryption key with the new
    /// password. This returns the new encrypted user key and the new password hash.
    pub async fn update_password(
        &self,
        new_password: SensitiveString,
    ) -> Result<UpdatePasswordResponse> {
        Ok(self
            .0
             .0
            .write()
            .await
            .crypto()
            .update_password(new_password)
            .await?)
    }

    /// Generates a PIN protected user key from the provided PIN. The result can be stored and later
    /// used to initialize another client instance by using the PIN and the PIN key with
    /// `initialize_user_crypto`.
    pub async fn derive_pin_key(&self, pin: SensitiveString) -> Result<DerivePinKeyResponse> {
        Ok(self.0 .0.write().await.crypto().derive_pin_key(pin).await?)
    }

    /// Derives the pin protected user key from encrypted pin. Used when pin requires master
    /// password on first unlock.
    pub async fn derive_pin_user_key(&self, encrypted_pin: EncString) -> Result<EncString> {
        Ok(self
            .0
             .0
            .write()
            .await
            .crypto()
            .derive_pin_user_key(encrypted_pin)
            .await?)
    }

    pub async fn enroll_admin_password_reset(
        &self,
        public_key: String,
    ) -> Result<AsymmetricEncString> {
        Ok(self
            .0
             .0
            .write()
            .await
            .crypto()
            .enroll_admin_password_reset(public_key)?)
    }
}
