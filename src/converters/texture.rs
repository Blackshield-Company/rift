use crate::config::TargetConfig;
use image::GenericImageView;
use std::path::{Path, PathBuf};

/// Convert a texture asset: resize, reformat, output to engine project dir.
/// Falls back to copying as-is for unsupported formats.
pub fn convert(source: &Path, target: &TargetConfig) -> crate::Result<PathBuf> {
    let fmt = &target.textures.format;
    let max_w = target.textures.max_width;
    let max_h = target.textures.max_height;

    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Determine output filename
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("asset");
    let output_name = format!("{}.{}", stem, fmt);
    let output_path = target.project.join(&output_name);

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Try to open with the image crate — if it fails, copy as-is
    let img_result = image::open(source);
    match img_result {
        Ok(img) => {
            let (w, h) = img.dimensions();
            let (new_w, new_h) = fit_within(w, h, max_w, max_h);

            let processed = if new_w != w || new_h != h {
                img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3)
            } else {
                img
            };

            match fmt.as_str() {
                "png" => processed.save(&output_path)?,
                "jpg" | "jpeg" => processed.save(&output_path)?,
                "webp" => processed.save(&output_path)?,
                "bmp" => processed.save(&output_path)?,
                "tiff" => processed.save(&output_path)?,
                _ => {
                    // Unknown target format, save as PNG
                    let png_path = output_path.with_extension("png");
                    processed.save(&png_path)?;
                    return Ok(png_path);
                }
            }
        }
        Err(_) => {
            // Format not supported by image crate (e.g., PSD without feature flag)
            // Fall back to copying as-is with the original extension
            let fallback_name = format!("{}.{}", stem, ext);
            let fallback_path = target.project.join(&fallback_name);
            if let Some(parent) = fallback_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(source, &fallback_path)?;
            return Ok(fallback_path);
        }
    }

    Ok(output_path)
}

/// Fit dimensions within max bounds, maintaining aspect ratio.
/// Both results stay within their bound and neither side is shrunk more than needed.
fn fit_within(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    if w <= max_w && h <= max_h {
        return (w, h);
    }
    let scale = (max_w as f64 / w as f64).min(max_h as f64 / h as f64);
    let new_w = ((w as f64 * scale).round() as u32).clamp(1, max_w.max(1));
    let new_h = ((h as f64 * scale).round() as u32).clamp(1, max_h.max(1));
    (new_w, new_h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fit_within_no_resize() {
        assert_eq!(fit_within(100, 100, 2048, 2048), (100, 100));
    }

    #[test]
    fn test_fit_within_downscale() {
        let (w, h) = fit_within(4096, 2048, 2048, 2048);
        assert!(w <= 2048);
        assert!(h <= 2048);
        assert!(w >= h); // wider than tall, ratio preserved roughly
    }

    #[test]
    fn test_fit_within_respects_both_bounds() {
        // Wide image into a short box: height used to come out at 682 (> 512).
        assert_eq!(fit_within(3000, 1000, 2048, 512), (1536, 512));
        // Tall image into a square box: used to shrink to 250x1000 instead of 512x2048.
        assert_eq!(fit_within(1000, 4000, 2048, 2048), (512, 2048));
        // Wide image into a tall box.
        assert_eq!(fit_within(4096, 1024, 1024, 4096), (1024, 256));
        // Only one side over the bound.
        assert_eq!(fit_within(2049, 100, 2048, 2048), (2048, 100));
        // Extreme aspect ratio never yields a zero side.
        assert_eq!(fit_within(100_000, 1, 1024, 1024), (1024, 1));
    }

    #[test]
    fn test_fit_within_square() {
        let (w, h) = fit_within(4000, 4000, 1024, 1024);
        assert_eq!(w, 1024);
        assert_eq!(h, 1024);
    }
}
