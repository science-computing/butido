#[derive(Queryable)]
pub struct Artifact {
    pub id: i32,
    pub path: String,
    pub released: bool,
}

