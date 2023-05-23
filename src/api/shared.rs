use r2d2_sqlite::SqliteConnectionManager;

pub struct Pagination {
    page: Option<i64>,
    per_page: Option<i8>,
}
