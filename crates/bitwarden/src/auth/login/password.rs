#[cfg(feature = "internal")]
use bitwarden_crypto::SensitiveString;
#[cfg(feature = "internal")]
use log::info;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "internal")]
use crate::{
    auth::{api::request::PasswordTokenRequest, login::TwoFactorRequest},
    client::{Kdf, LoginMethod},
    Client,
};
use crate::{
    auth::{
        api::response::IdentityTokenResponse,
        login::response::{captcha_response::CaptchaResponse, two_factor::TwoFactorProviders},
    },
    error::Result,
};

#[cfg(feature = "internal")]
pub(crate) async fn login_password(
    client: &mut Client,
    input: PasswordLoginRequest,
) -> Result<PasswordLoginResponse> {
    use bitwarden_crypto::{EncString, HashPurpose, MasterKey};

    use crate::{client::UserLoginMethod, error::require};

    info!("password logging in");

    let password_vec = input.password.into();

    let master_key = MasterKey::derive(&password_vec, input.email.as_bytes(), &input.kdf)?;
    let password_hash =
        master_key.derive_master_key_hash(&password_vec, HashPurpose::ServerAuthorization)?;

    let response = request_identity_tokens(
        client,
        &input.email,
        &input.two_factor,
        password_hash.expose(),
    )
    .await?;

    if let IdentityTokenResponse::Authenticated(r) = &response {
        client.set_tokens(
            r.access_token.clone(),
            r.refresh_token.clone(),
            r.expires_in,
        );
        client.set_login_method(LoginMethod::User(UserLoginMethod::Username {
            client_id: "web".to_owned(),
            email: input.email.to_owned(),
            kdf: input.kdf.to_owned(),
        }));

        let user_key: EncString = require!(r.key.as_deref()).parse()?;
        let private_key: EncString = require!(r.private_key.as_deref()).parse()?;

        client.initialize_user_crypto_master_key(master_key, user_key, private_key)?;
    }

    PasswordLoginResponse::process_response(response)
}

#[cfg(feature = "internal")]
async fn request_identity_tokens(
    client: &mut Client,
    email: &str,
    two_factor: &Option<TwoFactorRequest>,
    password_hash: &str,
) -> Result<IdentityTokenResponse> {
    use crate::client::client_settings::DeviceType;

    let config = client.get_api_configurations().await;
    PasswordTokenRequest::new(
        email,
        password_hash,
        DeviceType::ChromeBrowser,
        "b86dd6ab-4265-4ddf-a7f1-eb28d5677f33",
        two_factor,
    )
    .send(config)
    .await
}

#[cfg(feature = "internal")]
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
/// Login to Bitwarden with Username and Password
pub struct PasswordLoginRequest {
    /// Bitwarden account email address
    pub email: String,
    /// Bitwarden account master password
    pub password: SensitiveString,
    // Two-factor authentication
    pub two_factor: Option<TwoFactorRequest>,
    /// Kdf from prelogin
    pub kdf: Kdf,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PasswordLoginResponse {
    pub authenticated: bool,
    /// TODO: What does this do?
    pub reset_master_password: bool,
    /// Whether or not the user is required to update their master password
    pub force_password_reset: bool,
    /// The available two factor authentication options. Present only when authentication fails due
    /// to requiring a second authentication factor.
    pub two_factor: Option<TwoFactorProviders>,
    /// The information required to present the user with a captcha challenge. Only present when
    /// authentication fails due to requiring validation of a captcha challenge.
    pub captcha: Option<CaptchaResponse>,
}

impl PasswordLoginResponse {
    pub(crate) fn process_response(
        response: IdentityTokenResponse,
    ) -> Result<PasswordLoginResponse> {
        match response {
            IdentityTokenResponse::Authenticated(success) => Ok(PasswordLoginResponse {
                authenticated: true,
                reset_master_password: success.reset_master_password,
                force_password_reset: success.force_password_reset,
                two_factor: None,
                captcha: None,
            }),
            IdentityTokenResponse::Payload(_) => Ok(PasswordLoginResponse {
                authenticated: true,
                reset_master_password: false,
                force_password_reset: false,
                two_factor: None,
                captcha: None,
            }),
            IdentityTokenResponse::TwoFactorRequired(two_factor) => Ok(PasswordLoginResponse {
                authenticated: false,
                reset_master_password: false,
                force_password_reset: false,
                two_factor: Some(two_factor.two_factor_providers.into()),
                captcha: two_factor.captcha_token.map(Into::into),
            }),
            IdentityTokenResponse::CaptchaRequired(captcha) => Ok(PasswordLoginResponse {
                authenticated: false,
                reset_master_password: false,
                force_password_reset: false,
                two_factor: None,
                captcha: Some(captcha.site_key.into()),
            }),
            IdentityTokenResponse::Refreshed(_) => {
                unreachable!("Got a `refresh_token` answer to a login request")
            }
        }
    }
}
