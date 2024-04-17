use std::path::PathBuf;

use anyhow::Result;
use aws_sdk_s3::Client;
use clap::{Parser, Subcommand};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let shared_config = aws_config::load_from_env().await;
    let client = Client::new(&shared_config);
    cli.execute(client).await?;
    Ok(())
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(after_long_help = "Example: artifact-mover --bucket")]
struct Cli {
    #[arg(short, long, help = "Bucket to upload/download")]
    bucket: String,

    /// name of artifact
    #[arg(short, long, global = true, help = "Name of artifact")]
    name: String,

    /// path to artifact or directory
    #[arg(
        short,
        long,
        help = "Path to artifact or directory of artifact(s) or location for download"
    )]
    path: PathBuf,

    #[arg(long, help = "Unique identifier for this artifact")]
    identifier: String,

    #[arg(
        long,
        global = true,
        help = "Name of AWS profile to use, otherwise: 'default'"
    )]
    profile: Option<String>,

    #[arg(
        long,
        global = true,
        help = "Custom prefix to add to key used to store artifact(s) on S3, default to 'artifacts'"
    )]
    prefix_key: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Upload,
    Download,
}

impl Cli {
    pub async fn execute(&self, client: Client) -> Result<()> {
        match self.command {
            Commands::Upload => run_upload(&self, client).await,
            Commands::Download => run_download(&self, client).await,
        }
    }
}

async fn run_upload(cli: &Cli, client: Client) -> Result<()> {
    let mut buffer = vec![];
    {
        let mut archiver = tar::Builder::new(&mut buffer);
        match cli.path.is_file() {
            true => archiver.append_path(&cli.path)?,
            false => archiver.append_dir_all(&cli.path, ".")?,
        };
        archiver.finish()?;
    }
    let buffer = compress(&buffer)?;
    let digest = md5::compute(&buffer);

    let key = format!(
        "{}/{}/{:x}.tar.bz2",
        cli.prefix_key.as_ref().unwrap_or(&"artifacts".to_string()),
        cli.identifier.clone(),
        digest,
    );

    client
        .put_object()
        .bucket(&cli.bucket)
        .key(&key)
        .content_type("application/x-tar+bzip2")
        .body(buffer.into())
        .send()
        .await?;

    Ok(())
}

async fn run_download(cli: &Cli, client: Client) -> Result<()> {
    let prefix = format!(
        "{}/{}/",
        cli.prefix_key.as_ref().unwrap_or(&"artifacts".to_string()),
        cli.identifier.clone()
    );
    let mut resp = client
        .list_objects_v2()
        .bucket(&cli.bucket)
        .prefix(prefix)
        .into_paginator()
        .send();

    while let Some(result) = resp.next().await {
        let output = result?;
        for object in output.contents() {
            if let Some(key) = object.key() {
                if key.ends_with("tar.bz2") {
                    // TODO: Verify md5 hash in filename
                    let mut buffer = vec![];
                    let body = client
                        .get_object()
                        .bucket(&cli.bucket)
                        .key(key)
                        .send()
                        .await?;
                    tokio::io::copy_buf(&mut body.body.into_async_read(), &mut buffer).await?;
                    let buffer = decompress(&buffer)?;

                    let mut archive = tar::Archive::new(buffer.as_slice());
                    archive.set_overwrite(true);
                    archive.unpack(&cli.path)?;
                }
            }
        }
    }
    Ok(())
}

fn compress(buf: &[u8]) -> Result<Vec<u8>> {
    let mut out = vec![];
    libcramjam::bzip2::compress(buf, &mut out, None)?;
    Ok(out)
}
fn decompress(buf: &[u8]) -> Result<Vec<u8>> {
    let mut out = vec![];
    libcramjam::bzip2::decompress(buf, &mut out)?;
    Ok(out)
}
