use std::cmp::min;
use reqwest::Client;
use indicatif::{ProgressBar, ProgressStyle};
use futures_util::StreamExt;
use clap::Parser;
use url::Url;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = 39)]
    fedora: u8,

    #[arg(long, default_value = "x86_64")]
    arch: String,

    #[arg(long, default_value = "linux-firmware-20230919-1")]
    package: String,

    #[arg(long, default_value = "noarch")]
    package_arch: String,
}

async fn download_file(client: &Client, args: &Args, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let package_dict = args.package.chars().nth(0).unwrap();

    let package_url = format!("{}/Packages/{}/{}.fc{}.{}.rpm",
                              url,
                              package_dict,
                              args.package,
                              args.fedora,
                              args.package_arch);

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
        .template("                 [{wide_bar:.white/blue}] [{elapsed_precise}] {bytes}/{total_bytes} ({bytes_per_sec})")?
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
async fn list_of_mirrors(args: &Args) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();
    let url = format!("https://mirrors.fedoraproject.org/mirrorlist?repo=fedora-{}&arch={}",
                      args.fedora,
                      args.arch);
    let body = reqwest::get(url)
        .await?
        .text()
        .await?;
    let lines = body.split("\n");
    for line in lines {
        if line.starts_with("#") {
            println!("Mirrorlist config: {}", line);
        }
        if !line.starts_with("http") {
            continue;
        }
        result.push(line.to_string());
    }

    return Ok(result);
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mirrors = list_of_mirrors(&args).await.unwrap();
    for mirror in mirrors {
        let domain = Url::parse(mirror.as_str()).unwrap();
        println!("Checking mirror: {}", domain.host_str().unwrap());
        match download_file(&Client::new(), &args, mirror.as_str()).await {
            Ok(b) => b,
            Err(e) => {
                println!("{}", e);
                continue;
            }
        }
    }
}
