use std::sync::Arc;

use axum::{
    extract::{Path, Query},
    routing::{delete as DELETE, get as GET, post as POST},
    Json, Router,
};
use bitwarden::{
    secrets_manager::secrets::{SecretResponse, SecretsDeleteRequest, SecretsDeleteResponse},
    Client,
};
use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{command::secret_common, SecretCommand};

#[derive(Deserialize)]
struct SecretListRequest {
    project_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub enum SecretResult {
    List(Vec<SecretResponse>),
    Get(Vec<SecretResponse>),
    Create(Vec<SecretResponse>),
    Edit(Vec<SecretResponse>),
    Delete(Vec<SecretsDeleteResponse>),
}

// We just want to return the Secret*Response, instead of a nested List, Get, etc.
impl Serialize for SecretResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            SecretResult::List(vec)
            | SecretResult::Get(vec)
            | SecretResult::Create(vec)
            | SecretResult::Edit(vec) => vec.serialize(serializer),
            SecretResult::Delete(vec) => vec.serialize(serializer),
        }
    }
}

pub(crate) async fn serve(
    hostname: &str,
    port: u32,
    client: Client,
    organization_id: Uuid,
) -> Result<()> {
    eprintln!("hostname: {hostname}");
    eprintln!("port:     {port}");

    let client = Arc::new(client);
    let app = Router::new()
        .route("/", GET(Json(json!({"data": "bws REST API"}))))
        .route(
            "/secrets",
            GET({
                let client = Arc::clone(&client);
                move |params: Query<SecretListRequest>| async move {
                    secret_list_handler(params.0, &client, organization_id).await
                }
            }),
        )
        .route(
            "/secrets",
            POST({
                let client = Arc::clone(&client);
                move |Json(payload): Json<SecretListRequest>| async move {
                    secret_list_handler(payload, &client, organization_id).await
                }
            }),
        )
        .route(
            "/secrets",
            DELETE({
                let client = Arc::clone(&client);
                move |Json(payload): Json<SecretsDeleteRequest>| async move {
                    secrets_delete_handler(payload.ids, &client).await
                }
            }),
        )
        // each endpoint with a capture group will consume the Arc<Client>,
        // so they should go last to avoid unnecessary clones
        .route(
            "/secret/{secret_id}",
            GET(move |Path(secret_id): Path<Uuid>| {
                let client = Arc::clone(&client);
                async move { secret_get_handler(secret_id, &client).await }
            }),
        );

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", hostname, port)).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn secret_get_handler(secret_id: Uuid, client: &Arc<Client>) -> Json<serde_json::Value> {
    match process_request(crate::SecretCommand::Get { secret_id }, client, None).await {
        Ok(secrets) => Json(json!({ "data": secrets })),
        Err(err) => {
            eprintln!("Error processing secret request: {err}");
            Json(json!({ "error": err.to_string() }))
        }
    }
}

async fn secret_list_handler(
    payload: SecretListRequest,
    client: &Arc<Client>,
    organization_id: Uuid,
) -> Json<serde_json::Value> {
    let project_id = payload.project_id;

    match process_request(
        crate::SecretCommand::List { project_id },
        client,
        Some(organization_id),
    )
    .await
    {
        Ok(secrets) => Json(json!({ "data": secrets })),
        Err(err) => {
            eprintln!("Error processing secret list: {err}");
            Json(json!({ "error": err.to_string() }))
        }
    }
}

async fn secrets_delete_handler(
    secret_ids: Vec<Uuid>,
    client: &Arc<Client>,
) -> Json<serde_json::Value> {
    match process_request(crate::SecretCommand::Delete { secret_ids }, client, None).await {
        Ok(secrets) => Json(json!({ "data": secrets })),
        Err(err) => {
            eprintln!("Error processing secret request: {err}");
            Json(json!({ "error": err.to_string() }))
        }
    }
}

async fn process_request(
    command: SecretCommand,
    client: &Client,
    organization_id: Option<Uuid>,
) -> Result<SecretResult> {
    match command {
        SecretCommand::List { project_id } => {
            let secrets = secret_common::list(
                client,
                organization_id.expect("an organization ID is required to list secrets"),
                project_id,
            )
            .await?;
            Ok(SecretResult::List(secrets))
        }
        SecretCommand::Get { secret_id } => {
            let secret = secret_common::get(client, secret_id).await?;
            Ok(SecretResult::Get(vec![secret]))
        }
        SecretCommand::Create {
            key,
            value,
            note,
            project_id,
        } => {
            let created_secret = secret_common::create(
                client,
                organization_id.expect("an organization ID should be provided to create secrets"),
                secret_common::SecretCreateCommandModel {
                    key,
                    value,
                    note,
                    project_id,
                },
            )
            .await?;
            Ok(SecretResult::Create(vec![created_secret]))
        }
        SecretCommand::Edit {
            secret_id,
            key,
            value,
            note,
            project_id,
        } => {
            let edited_secret = secret_common::edit(
                client,
                organization_id.expect("an organization ID should be provided to edit a secret"),
                secret_common::SecretEditCommandModel {
                    id: secret_id,
                    key,
                    value,
                    note,
                    project_id,
                },
            )
            .await?;
            Ok(SecretResult::Edit(vec![edited_secret]))
        }
        SecretCommand::Delete { secret_ids } => {
            let result = secret_common::delete(client, secret_ids).await?;
            Ok(SecretResult::Delete(vec![result]))
        }
    }
}
