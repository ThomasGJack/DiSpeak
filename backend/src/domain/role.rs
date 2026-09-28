use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateRole {
    pub name: String,
    pub color: Option<String>,
    pub permissions_bitfield: Option<i32>,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateRole {
    pub name: Option<String>,
    pub color: Option<String>,
    pub permissions_bitfield: Option<i32>,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AssignRoles {
    pub role_ids: Vec<String>,
}

