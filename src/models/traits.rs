use uuid::Uuid;

pub trait HasId {
    fn get_id(&self) -> Option<Uuid>;
}