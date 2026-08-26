use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Default, Debug)]
pub struct SearchGroupResult {
    pub id: String,
    pub name: String,
    pub avatar: String,
}

#[derive(Clone, Deserialize, Serialize, Default, Debug)]
pub struct SearchUserResult {
    pub id: String,
    pub name: String,
    pub avatar: String,
}

#[derive(Clone, Deserialize, Serialize, Default, Debug)]
pub struct SearchResult {
    pub groups: Vec<SearchGroupResult>,
    pub users: Vec<SearchUserResult>,
}
