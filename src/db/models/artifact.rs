use diesel::prelude::*;

#[derive(Identifiable, Queryable, Associations)]
#[belongs_to(Job)]
pub struct Artifact {
    pub id: i32,
    pub path: String,
    pub released: bool,
    pub job_id: i32,
}

