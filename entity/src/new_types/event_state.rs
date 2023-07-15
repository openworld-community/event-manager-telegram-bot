use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use sea_orm::sea_query::{ArrayType, Nullable, ValueType, ValueTypeErr};
use sea_orm::{ColIdx, ColumnType, DbErr, QueryResult, TryGetError, TryGetable, Value};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, FromPrimitive)]
pub enum EventState {
    #[default]
    Open = 0,
    Close = 1,
}

impl From<EventState> for Value {
    fn from(value: EventState) -> Self {
        Value::Int(Some(value as i32))
    }
}

#[derive(Debug)]
struct ErrorToConvertFromI32(i32);

impl Display for ErrorToConvertFromI32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "error to convert from {} to {}",
            self.0,
            stringify!(EventState)
        )
    }
}

impl Error for ErrorToConvertFromI32 {}

impl TryGetable for EventState {
    fn try_get_by<I: ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
        let state: Option<i32> = res.try_get_by(index).map_err(TryGetError::DbErr)?;

        match state {
            Some(state) => match EventState::from_i32(state) {
                Some(state) => Ok(state),
                None => Err(TryGetError::DbErr(DbErr::TryIntoErr {
                    from: "i32",
                    into: stringify!(EventState),
                    source: Box::new(ErrorToConvertFromI32(state)),
                })),
            },
            None => Err(TryGetError::Null(format!(
                "can not get {} from index {:?}",
                stringify!(EventState),
                index
            ))),
        }
    }
}

impl ValueType for EventState {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        match v {
            Value::Int(Some(val)) => EventState::from_i32(val).ok_or(ValueTypeErr),
            _ => Err(ValueTypeErr),
        }
    }

    fn type_name() -> String {
        stringify!(EventState).to_owned()
    }

    fn array_type() -> ArrayType {
        ArrayType::Int
    }

    fn column_type() -> ColumnType {
        ColumnType::Integer
    }
}

impl Nullable for EventState {
    fn null() -> Value {
        Value::Int(None)
    }
}
