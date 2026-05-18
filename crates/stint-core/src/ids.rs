use uuid::Uuid;

pub fn new_local_uuid() -> String {
    Uuid::new_v4().to_string()
}
