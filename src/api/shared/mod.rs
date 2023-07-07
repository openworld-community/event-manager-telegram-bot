mod internal_server_error;
mod response_error;
mod validation_error;

use crate::util;
pub use internal_server_error::into_internal_server_error_response;
pub use internal_server_error::InternalServerError;
pub use internal_server_error::QueryError;
pub use response_error::AppError;
pub use validation_error::ValidationError;
use serde::Serialize;

#[derive(Deserialize)]
pub struct RawPagination {
    page: Option<i64>,
    per_page: Option<i64>,
}

pub trait Pagination {
    const DEFAULT_PAGE: i64 = 1;
    const DEFAULT_PER_PAGE: i64 = 20;
    const MAX_PER_PAGE: i64 = 150;

    fn page(&self) -> Option<i64>;
    fn per_page(&self) -> Option<i64>;

    fn filtered_per_page(&self) -> i64 {
        let val = self.per_page().unwrap_or(Self::DEFAULT_PER_PAGE);
        if val > Self::MAX_PER_PAGE {
            Self::MAX_PER_PAGE
        } else {
            val
        }
    }

    fn filtered_page(&self) -> i64 {
        self.page().unwrap_or(Self::DEFAULT_PAGE)
    }

    fn limit(&self) -> i64 {
        self.filtered_per_page()
    }

    fn offset(&self) -> i64 {
        (self.filtered_page() - 1) * self.filtered_per_page()
    }
}

impl Pagination for RawPagination {
    fn page(&self) -> Option<i64> {
        self.page
    }

    fn per_page(&self) -> Option<i64> {
        self.per_page
    }
}

#[derive(Serialize)]
pub struct WithId<Id: Serialize, WithoutId: Serialize> {
    pub(crate) id: Id,
    #[serde(flatten)]
    pub(crate) entity: WithoutId,
}
