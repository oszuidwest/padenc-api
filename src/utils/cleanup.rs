use crate::models::data::{Image, Program, Track};
use crate::utils::multipart::cleanup_image;

pub trait HasImage {
    fn get_image(&self) -> Option<&Image>;
}

impl HasImage for Program {
    fn get_image(&self) -> Option<&Image> {
        self.image.as_ref()
    }
}

impl HasImage for Track {
    fn get_image(&self) -> Option<&Image> {
        self.image.as_ref()
    }
}

pub fn cleanup_optional_data_image<T: HasImage>(data: &Option<T>) {
    if let Some(item) = data {
        if let Some(image) = item.get_image() {
            cleanup_image(&image.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::data::{Image, Item, Program, Track};
    use std::fs;
    use tempfile::tempdir;
    use uuid::Uuid;

    fn image_at(path: std::path::PathBuf) -> Image {
        Image {
            content_type: Some("image/jpeg".into()),
            path: Some(path),
            filename: Some("x.jpg".into()),
        }
    }

    fn track_with_image(image: Option<Image>) -> Track {
        Track {
            id: Uuid::new_v4(),
            item: Item { title: "T".into(), artist: None },
            expires_at: None,
            image,
        }
    }

    fn program_with_image(image: Option<Image>) -> Program {
        Program {
            id: Uuid::new_v4(),
            name: "P".into(),
            expires_at: None,
            image,
        }
    }

    #[test]
    fn has_image_impls_return_inner_image() {
        assert!(track_with_image(Some(image_at("/tmp/a.jpg".into()))).get_image().is_some());
        assert!(track_with_image(None).get_image().is_none());
        assert!(program_with_image(Some(image_at("/tmp/b.jpg".into()))).get_image().is_some());
        assert!(program_with_image(None).get_image().is_none());
    }

    #[test]
    fn cleanup_removes_existing_track_image() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("img.jpg");
        fs::write(&path, b"data").unwrap();
        assert!(path.exists());

        let data = Some(track_with_image(Some(image_at(path.clone()))));
        cleanup_optional_data_image(&data);
        assert!(!path.exists(), "image file should have been deleted");
    }

    #[test]
    fn cleanup_none_data_is_noop() {
        let data: Option<Track> = None;
        cleanup_optional_data_image(&data); // must not panic
    }

    #[test]
    fn cleanup_program_without_image_is_noop() {
        let data = Some(program_with_image(None));
        cleanup_optional_data_image(&data); // must not panic
    }
}