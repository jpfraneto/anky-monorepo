use anyhow::Result;
use std::path::Path;

/// Prepare training dataset by merging base images with newly generated Ankys.
/// Returns the path to the prepared dataset directory and the count of image-caption pairs.
pub fn prepare_dataset(
    base_dataset_dir: &Path,
    generated_images_dir: &Path,
    output_dir: &Path,
) -> Result<(String, usize)> {
    std::fs::create_dir_all(output_dir)?;

    let mut count = 0;

    // Copy base dataset images + captions
    if base_dataset_dir.exists() {
        for entry in std::fs::read_dir(base_dataset_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if matches!(ext.to_str(), Some("png" | "jpg" | "jpeg" | "webp")) {
                    let caption_path = path.with_extension("txt");
                    if caption_path.exists() {
                        let dest_img = output_dir.join(entry.file_name());
                        let dest_cap = output_dir.join(caption_path.file_name().unwrap());
                        std::fs::copy(&path, &dest_img)?;
                        std::fs::copy(&caption_path, &dest_cap)?;
                        count += 1;
                    }
                }
            }
        }
    }

    // Copy generated Anky images + captions
    if generated_images_dir.exists() {
        for entry in std::fs::read_dir(generated_images_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if matches!(ext.to_str(), Some("png" | "jpg")) {
                    let caption_path = path.with_extension("txt");
                    if caption_path.exists() {
                        let dest_img = output_dir.join(entry.file_name());
                        let dest_cap = output_dir.join(caption_path.file_name().unwrap());
                        std::fs::copy(&path, &dest_img)?;
                        std::fs::copy(&caption_path, &dest_cap)?;
                        count += 1;
                    }
                }
            }
        }
    }

    tracing::info!("Prepared dataset: {} image-caption pairs in {}", count, output_dir.display());
    Ok((output_dir.to_string_lossy().to_string(), count))
}
