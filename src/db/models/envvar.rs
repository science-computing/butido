#[derive(Queryable)]
pub struct EnvVar {
    pub id: i32,
    pub name: String,
    pub value: String,
}
