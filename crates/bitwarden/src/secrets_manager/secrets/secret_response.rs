use bitwarden_api_api::models::{
    BaseSecretResponseModel, BaseSecretResponseModelListResponseModel, SecretResponseModel,
};
use bitwarden_crypto::{Decryptable, DecryptedString, EncString};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    client::encryption_settings::EncryptionSettings,
    error::{require, Result},
};

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub project_id: Option<Uuid>,

    pub key: String,
    pub value: String,
    pub note: String,

    pub creation_date: DateTime<Utc>,
    pub revision_date: DateTime<Utc>,
}

impl SecretResponse {
    pub(crate) fn process_response(
        response: SecretResponseModel,
        enc: &EncryptionSettings,
    ) -> Result<SecretResponse> {
        let base = BaseSecretResponseModel {
            object: response.object,
            id: response.id,
            organization_id: response.organization_id,
            key: response.key,
            value: response.value,
            note: response.note,
            creation_date: response.creation_date,
            revision_date: response.revision_date,
            projects: response.projects,
        };
        Self::process_base_response(base, enc)
    }
    pub(crate) fn process_base_response(
        response: BaseSecretResponseModel,
        enc: &EncryptionSettings,
    ) -> Result<SecretResponse> {
        let org_id = response.organization_id;

        let key: DecryptedString = require!(response.key)
            .parse::<EncString>()?
            .decrypt(enc, &org_id)?;
        let value: DecryptedString = require!(response.value)
            .parse::<EncString>()?
            .decrypt(enc, &org_id)?;
        let note: DecryptedString = require!(response.note)
            .parse::<EncString>()?
            .decrypt(enc, &org_id)?;

        let project = response
            .projects
            .and_then(|p| p.into_iter().next())
            .and_then(|p| p.id);

        Ok(SecretResponse {
            id: require!(response.id),
            organization_id: require!(org_id),
            project_id: project,
            key: key.expose().to_owned(),
            value: value.expose().to_owned(),
            note: note.expose().to_owned(),

            creation_date: require!(response.creation_date).parse()?,
            revision_date: require!(response.revision_date).parse()?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretsResponse {
    pub data: Vec<SecretResponse>,
}

impl SecretsResponse {
    pub(crate) fn process_response(
        response: BaseSecretResponseModelListResponseModel,
        enc: &EncryptionSettings,
    ) -> Result<SecretsResponse> {
        Ok(SecretsResponse {
            data: response
                .data
                .unwrap_or_default()
                .into_iter()
                .map(|r| SecretResponse::process_base_response(r, enc))
                .collect::<Result<_, _>>()?,
        })
    }
}
