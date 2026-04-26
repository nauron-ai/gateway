mod audit;
mod audit_md;
mod envfile;
mod gateway_client;
mod repo_root;
mod scan;
mod upload;

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use clap::Parser;

use crate::audit::{AuditConfig, AuditState};
use crate::gateway_client::GatewayClient;
use crate::repo_root::resolve_repo_root_path;
use crate::scan::collect_files;
use crate::upload::{UploadConfig, UploadOutcome, upload_one};

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "https://gateway.nauron.ai")]
    base_url: String,
    #[arg(long)]
    env_file: Option<PathBuf>,
    #[arg(long)]
    admin_email: Option<String>,
    #[arg(long)]
    admin_password: Option<String>,
    #[arg(long)]
    input_dir: PathBuf,
    #[arg(long)]
    title: Option<String>,
    #[arg(long)]
    description: Option<String>,
    #[arg(long)]
    audit_file: Option<PathBuf>,
    #[arg(long, default_value_t = 200 * 1024 * 1024)]
    max_bytes: u64,
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let env_file = match args.env_file.clone() {
        Some(value) => value,
        None => resolve_repo_root_path(".env.gateway")?,
    };
    let audit_file = match args.audit_file.clone() {
        Some(value) => value,
        None => resolve_repo_root_path("audit-progress.md")?,
    };
    let title = match args.title.clone() {
        Some(value) => value,
        None => derive_title(&args.input_dir),
    };
    let description = match args.description.clone() {
        Some(value) => value,
        None => format!("Import z {} (rekurencyjnie)", args.input_dir.display()),
    };

    let env = envfile::read_env_file(&env_file)?;
    let admin_email = envfile::resolve_value(args.admin_email, &env, "ADMIN_EMAIL")?;
    let admin_password = envfile::resolve_value(args.admin_password, &env, "ADMIN_PASSWORD")?;

    let client = GatewayClient::new(args.base_url.clone())?;
    let token = client.login(&admin_email, &admin_password).await?;

    let audit_config = AuditConfig {
        audit_path: audit_file,
        base_url: args.base_url.clone(),
        input_dir: args.input_dir.clone(),
        title: title.clone(),
        description: description.clone(),
        mode: "rdf".to_string(),
    };
    let mut audit_state = AuditState::load_or_init(audit_config)?;
    let context_id = match audit_state.context_id() {
        Some(existing) => existing,
        None => {
            let id = client.create_context(&token, "rdf").await?;
            client
                .update_context(&token, id, &title, &description)
                .await?;
            audit_state.persist_context(id)?;
            id
        }
    };

    let allowed_exts: BTreeSet<String> = ["pdf", "txt", "jpg", "jpeg", "png"]
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    let files = collect_files(&args.input_dir, &allowed_exts)?;

    let upload_config = UploadConfig {
        token,
        context_id,
        max_bytes: args.max_bytes,
        dry_run: args.dry_run,
    };

    let mut row_index = audit_state.next_row_index();
    for path in files {
        let rel_path = audit::to_rel_path(&args.input_dir, &path);
        let size_bytes = match fs::metadata(&path) {
            Ok(meta) => meta.len(),
            Err(_) => 0,
        };
        if audit_state.is_done(&rel_path, size_bytes) {
            continue;
        }

        let outcome = match upload_one(&client, &upload_config, &path).await {
            Ok(response) => UploadOutcome::Uploaded(response),
            Err(err) => UploadOutcome::from(err),
        };

        audit_state.append_row(row_index, &rel_path, &path, context_id, outcome)?;
        row_index += 1;
    }

    Ok(())
}

fn derive_title(input_dir: &std::path::Path) -> String {
    match input_dir.file_name().and_then(|s| s.to_str()) {
        Some(name) if !name.trim().is_empty() => name.to_string(),
        _ => input_dir.to_string_lossy().to_string(),
    }
}
