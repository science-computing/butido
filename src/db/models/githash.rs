use crate::schema::githashes;

#[derive(Queryable)]
pub struct GitHash {
    pub id: i32,
    pub hash: String,
}

#[derive(Insertable)]
#[table_name="githashes"]
pub struct NewGitHash<'a> {
    pub hash: &'a str,
}

