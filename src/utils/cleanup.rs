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