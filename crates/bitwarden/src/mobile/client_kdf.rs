use bitwarden_crypto::{HashPurpose, SensitiveString};

use crate::{client::Kdf, error::Result, mobile::kdf::hash_password, Client};

pub struct ClientKdf<'a> {
    pub(crate) client: &'a crate::Client,
}

impl<'a> ClientKdf<'a> {
    pub async fn hash_password(
        &self,
        email: String,
        password: SensitiveString,
        kdf_params: Kdf,
        purpose: HashPurpose,
    ) -> Result<SensitiveString> {
        hash_password(self.client, email, password, kdf_params, purpose).await
    }
}

impl<'a> Client {
    pub fn kdf(&'a self) -> ClientKdf<'a> {
        ClientKdf { client: self }
    }
}
