use std::cmp::min;
use reqwest::Client;
use indicatif::{ProgressBar, ProgressStyle};
use futures_util::StreamExt;

async fn download_file(client: &Client, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: get this dynamically somehow
    let package = "/Packages/l/linux-firmware-20230919-1.fc39.noarch.rpm";
    let package_url = format!("{}{}", url, package);

    let res = client
        .get(package_url.as_str())
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &package_url)))?;
    let total_size = res
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", &package_url))?;
   

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("                 [{wide_bar:.white/blue}] {bytes}/{total_bytes} ({bytes_per_sec}) [{elapsed_precise}]")?
        .progress_chars("█  "));

    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();
    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Error while downloading")))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish();

    // TODO: use pb.per_sec() to keep track of download rate of each mirror and pretty print at the
    // end.
    return Ok(());
}

// TODO: refactor to idiomatic rust
async fn list_of_mirrors() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();

    // TODO: parameterize repo and arch
    let body = reqwest::get("https://mirrors.fedoraproject.org/mirrorlist?repo=fedora-39&arch=x86_64")
        .await?
        .text()
        .await?;
    let lines = body.split("\n");
    for line in lines {
        if line.starts_with("#") {
            continue;
        }
        result.push(line.to_string());
    }

    return Ok(result);
}

// TODO: setup clap 
#[tokio::main]
async fn main() {
    let mirrors = list_of_mirrors().await.unwrap();
    for mirror in mirrors {
        println!("Checking mirror: {}", mirror);
        download_file(&Client::new(), mirror.as_str()).await.unwrap();
    }
}
