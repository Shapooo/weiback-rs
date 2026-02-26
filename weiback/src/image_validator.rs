use image::{DynamicImage, GenericImageView, Pixel};
use img_hash::{HashAlg, HasherConfig};
use log::debug;

use crate::error::{Error, Result};

// 引入由 build.rs 在编译期生成的常量
include!(concat!(env!("OUT_DIR"), "/invalid_consts.rs"));

/// 允许 R/G/B 通道间的微小差异 (应对 JPEG 压缩失真)
const RGB_DIFF_TOLERANCE: u8 = 3;
/// 对采样得到的范围进行微量外扩，增加兼容性
const RANGE_BUFFER: u8 = 2;
/// 感知哈希的比对阈值，汉明距离小于此值认为匹配
const PHASH_THRESHOLD: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageStatus {
    Valid,
    Invalid,
}

pub struct ImageValidator {
    hasher: img_hash::Hasher,
}

impl ImageValidator {
    pub fn new() -> Self {
        let hasher = HasherConfig::new()
            .hash_alg(HashAlg::Gradient)
            .hash_size(16, 16)
            .to_hasher();
        Self { hasher }
    }

    /// 检测图片是否为失效占位图
    pub fn is_invalid_weibo_image(&self, data: &[u8]) -> Result<ImageStatus> {
        // 使用 Cursor 和 guess_format 来兼容非标准头部或格式不匹配的图片
        let reader = image::io::Reader::new(std::io::Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| Error::FormatError(format!("IO error: {}", e)))?;

        let img = reader
            .decode()
            .map_err(|e| Error::FormatError(format!("Failed to decode image: {}", e)))?;

        let (width, height) = img.dimensions();
        if width < 120 || height < 120 {
            return Ok(ImageStatus::Valid);
        }

        // --- Stage 1: 颜色采样过滤 (使用动态生成的范围) ---
        if !self.check_stage1_color(&img, width, height) {
            return Ok(ImageStatus::Valid);
        }
        debug!(
            "Image passed Stage 1: Color match within range [{}, {}]",
            SAMPLE_GRAY_MIN.saturating_sub(RANGE_BUFFER),
            SAMPLE_GRAY_MAX.saturating_add(RANGE_BUFFER)
        );

        // --- Stage 2: 局部平滑度检测 (Variance) ---
        if !self.check_stage2_flatness(&img, width, height) {
            return Ok(ImageStatus::Valid);
        }
        debug!("Image passed Stage 2: Flatness match");

        // --- Stage 3: 感知哈希结构匹配 (Final Decision) ---
        if self.check_stage3_phash(&img) {
            debug!("Image passed Stage 3: pHash match. Marking as INVALID.");
            return Ok(ImageStatus::Invalid);
        }

        Ok(ImageStatus::Valid)
    }

    /// Stage 1: 检查边缘采样点是否符合预设的“微博失效灰”
    fn check_stage1_color(&self, img: &DynamicImage, w: u32, h: u32) -> bool {
        // 根据样本范围动态确定的边界
        let min = SAMPLE_GRAY_MIN.saturating_sub(RANGE_BUFFER);
        let max = SAMPLE_GRAY_MAX.saturating_add(RANGE_BUFFER);

        let samples = [
            (w / 10, h / 10),
            (w * 9 / 10, h / 10),
            (w / 10, h * 9 / 10),
            (w * 9 / 10, h * 9 / 10),
            (w / 2, h / 10),
            (w / 2, h * 9 / 10),
        ];

        for (x, y) in samples {
            let rgb = img.get_pixel(x, y).to_rgb().0;

            // 检查 R 是否在基准范围内
            if rgb[0] < min || rgb[0] > max {
                return false;
            }
            // 检查 R, G, B 是否足够接近 (判断是否为灰度)
            let is_gray = (rgb[0] as i16 - rgb[1] as i16).abs() <= RGB_DIFF_TOLERANCE as i16
                && (rgb[1] as i16 - rgb[2] as i16).abs() <= RGB_DIFF_TOLERANCE as i16;

            if !is_gray {
                return false;
            }
        }
        true
    }

    /// Stage 2: 检查背景平滑度（方差）
    fn check_stage2_flatness(&self, img: &DynamicImage, _w: u32, _h: u32) -> bool {
        let box_size = 32;
        let mut sum = 0.0;
        let mut sum_sq = 0.0;
        let n = (box_size * box_size) as f64;

        for y in 0..box_size {
            for x in 0..box_size {
                let luma = img.get_pixel(x, y).to_luma().0[0] as f64;
                sum += luma;
                sum_sq += luma * luma;
            }
        }

        let mean = sum / n;
        let variance = (sum_sq / n) - (mean * mean);

        // 合成图方差极低，真实照片由于噪点通常 > 0.5
        variance < 0.2
    }

    /// Stage 3: 感知哈希比对
    fn check_stage3_phash(&self, img: &DynamicImage) -> bool {
        let current_hash = self.hasher.hash_image(img);

        for base64_hash in INVALID_HASHES {
            if let Ok(target_hash) = img_hash::ImageHash::from_base64(base64_hash)
                && current_hash.dist(&target_hash) <= PHASH_THRESHOLD
            {
                return true;
            }
        }
        false
    }
}

impl Default for ImageValidator {
    fn default() -> Self {
        Self::new()
    }
}
