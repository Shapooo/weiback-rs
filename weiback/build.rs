//! Build script to generate constants for image validation.
//!
//! This script scans the `resources/samples/invalid` directory for sample images that
//! Weibo uses as placeholders for deleted or invalid content. It performs the following:
//!
//! 1.  **Perceptual Hashing**: Calculates `img_hash` values for these samples to enable
//!     fuzzy detection of similar invalid images in the main application.
//! 2.  **Color Sampling**: Analyzes the grayscale values (Red channel) of these images
//!     to determine the typical intensity range of the placeholder backgrounds.
//! 3.  **Code Generation**: Writes the results into `invalid_consts.rs` in the build
//!     output directory (`OUT_DIR`), defining `SAMPLE_GRAY_MIN`, `SAMPLE_GRAY_MAX`,
//!     and `INVALID_HASHES`.
//!
//! These generated constants are used by the `image_validator` module to identify
//! and filter out "image deleted" placeholders during media processing.

use std::env;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use image::GenericImageView;
use img_hash::{HashAlg, HasherConfig};

fn main() {
    println!("cargo:rerun-if-changed=resources/samples/invalid");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("invalid_consts.rs");

    let sample_dir = Path::new("resources/samples/invalid");
    let mut hashes = Vec::new();

    let mut min_r = 255u8;
    let mut max_r = 0u8;
    let mut found_samples = false;

    let hasher = HasherConfig::new()
        .hash_alg(HashAlg::Gradient)
        .hash_size(16, 16)
        .to_hasher();

    if sample_dir.exists()
        && let Ok(entries) = fs::read_dir(sample_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                // 尝试更稳健的读取方式
                let file_data = fs::read(&path).expect("Read sample file failed");

                // 使用 guess_format 自动识别，处理 SOI 标记缺失或格式不匹配
                let reader = image::io::Reader::new(Cursor::new(&file_data))
                    .with_guessed_format()
                    .expect("Cursor io error");

                if let Ok(img) = reader.decode() {
                    found_samples = true;
                    // 提取感知哈希
                    hashes.push(hasher.hash_image(&img).to_base64());

                    // 密集网格采样 (20x20 = 400个点)
                    let (w, h) = img.dimensions();
                    let step_x = (w / 20).max(1);
                    let step_y = (h / 20).max(1);

                    for y in (0..h).step_by(step_y as usize).take(20) {
                        for x in (0..w).step_by(step_x as usize).take(20) {
                            let r = img.get_pixel(x, y)[0];
                            // 过滤掉可能的纯黑或纯白点（避免采样到水印边缘的极端值）
                            if r > 100 && r < 250 {
                                if r < min_r {
                                    min_r = r;
                                }
                                if r > max_r {
                                    max_r = r;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 如果没有采样到，使用默认安全值
    if !found_samples {
        min_r = 230;
        max_r = 240;
    }

    let content = format!(
        "pub const SAMPLE_GRAY_MIN: u8 = {};\n\
         pub const SAMPLE_GRAY_MAX: u8 = {};\n\
         pub const INVALID_HASHES: &[&str] = &{:?};\n",
        min_r, max_r, hashes
    );

    fs::write(dest_path, content).expect("Failed to write constants file");
}
