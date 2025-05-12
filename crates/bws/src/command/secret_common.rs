use bitwarden::{
    secrets_manager::{
        secrets::{
            SecretCreateRequest, SecretGetRequest, SecretIdentifiersByProjectRequest,
            SecretIdentifiersRequest, SecretPutRequest, SecretResponse, SecretsDeleteRequest,
            SecretsDeleteResponse, SecretsGetRequest,
        },
        ClientSecretsExt,
    },
    Client,
};
use color_eyre::eyre::Result;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SecretCreateCommandModel {
    pub key: String,
    pub value: String,
    pub note: Option<String>,
    pub project_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct SecretEditCommandModel {
    pub id: Uuid,
    pub key: Option<String>,
    pub value: Option<String>,
    pub note: Option<String>,
    pub project_id: Option<Uuid>,
}

pub async fn list(
    client: &Client,
    organization_id: Uuid,
    project_id: Option<Uuid>,
) -> Result<Vec<SecretResponse>> {
    let res = if let Some(project_id) = project_id {
        client
            .secrets()
            .list_by_project(&SecretIdentifiersByProjectRequest { project_id })
            .await?
    } else {
        client
            .secrets()
            .list(&SecretIdentifiersRequest { organization_id })
            .await?
    };

    let secret_ids = res.data.into_iter().map(|e| e.id).collect();
    let secrets = client
        .secrets()
        .get_by_ids(SecretsGetRequest { ids: secret_ids })
        .await?
        .data;

    Ok(secrets)
}

pub async fn get(client: &Client, secret_id: Uuid) -> Result<SecretResponse> {
    let secret = client
        .secrets()
        .get(&SecretGetRequest { id: secret_id })
        .await?;

    Ok(secret)
}

pub async fn create(
    client: &Client,
    organization_id: Uuid,
    secret: SecretCreateCommandModel,
) -> Result<SecretResponse> {
    let secret = client
        .secrets()
        .create(&SecretCreateRequest {
            organization_id,
            key: secret.key,
            value: secret.value,
            note: secret.note.unwrap_or_default(),
            project_ids: Some(vec![secret.project_id]),
        })
        .await?;

    Ok(secret)
}

pub async fn edit(
    client: &Client,
    organization_id: Uuid,
    secret: SecretEditCommandModel,
) -> Result<SecretResponse> {
    let old_secret = client
        .secrets()
        .get(&SecretGetRequest { id: secret.id })
        .await?;

    let new_secret = client
        .secrets()
        .update(&SecretPutRequest {
            id: secret.id,
            organization_id,
            key: secret.key.unwrap_or(old_secret.key),
            value: secret.value.unwrap_or(old_secret.value),
            note: secret.note.unwrap_or(old_secret.note),
            project_ids: secret
                .project_id
                .or(old_secret.project_id)
                .map(|id| vec![id]),
        })
        .await?;

    Ok(new_secret)
}

pub async fn delete(client: &Client, secret_ids: Vec<Uuid>) -> Result<SecretsDeleteResponse> {
    let result = client
        .secrets()
        .delete(SecretsDeleteRequest { ids: secret_ids })
        .await?;

    Ok(result)
}
