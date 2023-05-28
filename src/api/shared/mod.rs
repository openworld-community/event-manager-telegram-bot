mod internal_server_error;

use crate::util;
pub use internal_server_error::into_internal_server_error_responce;
pub use internal_server_error::InternalServerError;
pub use internal_server_error::QueryError;
use serde::Serialize;

#[derive(Deserialize)]
pub struct RawPagination {
    page: Option<i64>,
    per_page: Option<i64>,
}

pub struct Pagination {
    page: i64,
    per_page: i64,
}

impl Pagination {
    pub fn limit(&self) -> i64 {
        self.per_page
    }
    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.per_page
    }
}

const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PER_PAGE: i64 = 20;
const MAX_PER_PAGE: i64 = 150;

impl From<RawPagination> for Pagination {
    fn from(value: RawPagination) -> Self {
        let per_page = value.per_page.unwrap_or(DEFAULT_PER_PAGE);
        Pagination {
            page: value.page.unwrap_or(DEFAULT_PAGE),
            per_page: util::max_or_value(per_page, MAX_PER_PAGE),
        }
    }
}

#[derive(Serialize)]
pub struct WithId<Id: Serialize, WithoutId: Serialize> {
    pub(crate) id: Id,
    #[serde(flatten)]
    pub(crate) entity: WithoutId,
}
