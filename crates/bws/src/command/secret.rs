use bitwarden::Client;
use color_eyre::eyre::{bail, Result};
use uuid::Uuid;

use super::secret_common;
use crate::{
    render::{serialize_response, OutputSettings},
    SecretCommand,
};

pub(crate) async fn process_command(
    command: SecretCommand,
    client: Client,
    organization_id: Uuid,
    output_settings: OutputSettings,
) -> Result<()> {
    match command {
        SecretCommand::List { project_id } => {
            let secrets = secret_common::list(&client, organization_id, project_id).await?;
            serialize_response(secrets, output_settings);
            Ok(())
        }
        SecretCommand::Get { secret_id } => {
            let secret = secret_common::get(&client, secret_id).await?;
            serialize_response(secret, output_settings);
            Ok(())
        }
        SecretCommand::Create {
            key,
            value,
            note,
            project_id,
        } => {
            let created_secret = secret_common::create(
                &client,
                organization_id,
                secret_common::SecretCreateCommandModel {
                    key,
                    value,
                    note,
                    project_id,
                },
            )
            .await?;
            serialize_response(created_secret, output_settings);
            Ok(())
        }
        SecretCommand::Edit {
            secret_id,
            key,
            value,
            note,
            project_id,
        } => {
            let edited_secret = secret_common::edit(
                &client,
                organization_id,
                secret_common::SecretEditCommandModel {
                    id: secret_id,
                    key,
                    value,
                    note,
                    project_id,
                },
            )
            .await?;
            serialize_response(edited_secret, output_settings);
            Ok(())
        }
        SecretCommand::Delete { secret_ids } => {
            let count = secret_ids.len();
            let result = secret_common::delete(&client, secret_ids).await?;

            let secrets_failed: Vec<(Uuid, String)> = result
                .data
                .into_iter()
                .filter_map(|r| r.error.map(|e| (r.id, e)))
                .collect();
            let deleted_secrets = count - secrets_failed.len();

            match deleted_secrets {
                2.. => println!("{} secrets deleted successfully.", deleted_secrets),
                1 => println!("{} secret deleted successfully.", deleted_secrets),
                _ => (),
            }

            match secrets_failed.len() {
                2.. => eprintln!("{} secrets had errors:", secrets_failed.len()),
                1 => eprintln!("{} secret had an error:", secrets_failed.len()),
                _ => (),
            }

            for secret in &secrets_failed {
                eprintln!("{}: {}", secret.0, secret.1);
            }

            if !secrets_failed.is_empty() {
                bail!("Errors when attempting to delete secrets.");
            }

            Ok(())
        }
    }
}
