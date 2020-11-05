use crate::schema::packages;

#[derive(Queryable)]
pub struct Package {
    pub id: i32,
    pub name: String,
    pub version: String,
}

#[derive(Insertable)]
#[table_name="packages"]
pub struct NewPackage<'a> {
    pub name: &'a str,
    pub version: &'a str,
}

