use clap::Parser;
use std::path::PathBuf;
use tokio::fs;
use walkdir::WalkDir;
use weiback::image_validator::{ImageStatus, ImageValidator};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory to scan for images
    #[arg(short, long)]
    directory: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    let validator = ImageValidator::new();

    println!("Scanning directory: {:?}", args.directory);

    for entry in WalkDir::new(args.directory)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "gif" => {
                match fs::read(path).await {
                    Ok(data) => {
                        if data.len() > 20_000 {
                            continue;
                        }
                        match validator.is_invalid_weibo_image(&data) {
                            Ok(ImageStatus::Invalid) => {
                                println!("Invalid image found: {}", path.display());
                            }
                            Ok(ImageStatus::Valid) => {
                                // It's a valid image, do nothing.
                            }
                            Err(e) => {
                                eprintln!("Error processing image {}: {}", path.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read file {}: {}", path.display(), e);
                    }
                }
            }
            _ => {
                // Not an image file, skip.
            }
        }
    }

    println!("Scan finished.");
    Ok(())
}
