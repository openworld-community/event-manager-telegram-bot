pub struct Pagination {
    page: Option<i64>,
    per_page: Option<i8>,
}

#[derive(Serialize)]
pub struct WithId<Id, WithoutId> {
    pub(crate) id: Id,
    #[serde(flatten)]
    pub(crate) entity: WithoutId,
}
