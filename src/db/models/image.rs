use crate::schema::images;

#[derive(Queryable)]
pub struct Image {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable)]
#[table_name="images"]
pub struct NewImage<'a> {
    pub name: &'a str,
}

