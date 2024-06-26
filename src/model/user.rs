use uuid:Uuid;

#[derive(Debug, Clonse)]
pub struct User {
    pub id: Uuid,
    pub name: String,
}

impl User {
    pub fn new(id: Uuid, name: &str) -> Self {
        User {
            id, 
            name: String::from(name)
        }
    }
}
