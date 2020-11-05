use chrono::NaiveDateTime;
use diesel::types::Uuid;
use diesel::types::Jsonb;

#[derive(Queryable)]
pub struct Submit {
    pub id: i32,
    pub uuid: Uuid,
    pub submit_time: NaiveDateTime,
    pub requested_image_id: i32,
    pub requested_package_id: i32,
    pub repo_hash_id: i32,
    pub tree: Jsonb,
    pub buildplan: Jsonb,
}

