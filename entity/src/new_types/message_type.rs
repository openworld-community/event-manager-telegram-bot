use std::error::Error;
use std::fmt::{Display, Formatter};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use sea_orm::{ColIdx, ColumnType, DbErr, QueryResult, TryGetable, TryGetError, Value};
use sea_orm::sea_query::{ArrayType, Nullable, ValueType, ValueTypeErr};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, FromPrimitive)]
pub enum MessageType {
    Direct = 0,
    Reminder = 1,
    WaitingListPrompt = 2,
}

impl From<MessageType> for Value {
    fn from(value: MessageType) -> Self {
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
            stringify!(MessageType)
        )
    }
}

impl Error for ErrorToConvertFromI32 {}

impl TryGetable for MessageType {
    fn try_get_by<I: ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
        let state: Option<i32> = res.try_get_by(index).map_err(TryGetError::DbErr)?;

        match state {
            Some(state) => match MessageType::from_i32(state) {
                Some(state) => Ok(state),
                None => Err(TryGetError::DbErr(DbErr::TryIntoErr {
                    from: "i32",
                    into: stringify!(MessageType),
                    source: Box::new(ErrorToConvertFromI32(state)),
                })),
            },
            None => Err(
                TryGetError::Null(format!("can not get {} from index {:?}", stringify!(MessageType), index))
            ),
        }
    }
}

impl ValueType for MessageType {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        match v {
            Value::Int(Some(value)) => {
                MessageType::from_i32(value).ok_or(ValueTypeErr)
            }
            _ => Err(ValueTypeErr),
        }
    }

    fn type_name() -> String {
        stringify!(MessageType).to_owned()
    }

    fn array_type() -> ArrayType {
        ArrayType::Int
    }

    fn column_type() -> ColumnType {
        ColumnType::Integer
    }
}

impl Nullable for MessageType {
    fn null() -> Value {
        Value::Int(None)
    }
}
