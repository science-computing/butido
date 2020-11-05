#[derive(Queryable)]
pub struct Package {
    pub id: i32,
    pub name: String,
    pub version: String,
}

