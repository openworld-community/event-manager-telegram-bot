#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct UserCred {
    pub user_name: String,
    pub password: String,
}

impl PartialEq<Self> for UserCred {
    fn eq(&self, other: &Self) -> bool {
        self.user_name == other.user_name && self.password == other.password
    }
}

impl Eq for UserCred {}
