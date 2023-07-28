use sea_orm::FromQueryResult;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement};

#[derive(FromQueryResult)]
pub struct VacantData {
    pub id: i32,
    pub max_adults: i32,
    pub max_children: i32,
    pub adults: i32,
    pub children: i32,
}

impl VacantData {
    pub fn get_vacant_adults(&self) -> i32 {
        self.max_adults - self.adults
    }

    pub fn get_vacant_children(&self) -> i32 {
        self.max_children - self.children
    }

    pub fn is_have_vacancies(&self) -> bool {
        self.get_vacant_adults() + self.get_vacant_children() > 0
    }
}

pub async fn get_vacancies<C>(event_id: i32, conn: &C) -> Result<Option<VacantData>, DbErr>
where
    C: ConnectionTrait,
{
    VacantData::find_by_statement(
         Statement::from_sql_and_values(
             conn.get_database_backend(),
             r#"SELECT a.max_adults, a.max_children, b.adults, b.children, a.id FROM events as a \
        LEFT JOIN (SELECT sum(adults) as adults, sum(children) as children, event FROM reservations WHERE event = ?1 AND waiting_list = 0 group by event) as b \
        ON a.id = b.event WHERE id = $1 group by id"#,
             [event_id.into()]
         )
     ).one(conn).await
}
