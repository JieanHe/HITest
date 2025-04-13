use serde::{de::Error as DError, Deserialize, Deserializer};

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Eq(String),
    Ne(String),
}

impl<'de> Deserialize<'de> for Condition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Value {
            Number(i64),
            String(String),
        }

        #[derive(Deserialize)]
        struct Helper {
            expect_eq: Option<Value>,
            expect_ne: Option<Value>,
        }

        let helper = Helper::deserialize(deserializer)?;

        let to_string = |value: Value| -> String {
            match value {
                Value::Number(n) => n.to_string(),
                Value::String(s) => s,
            }
        };

        match (helper.expect_eq, helper.expect_ne) {
            (Some(eq), None) => Ok(Condition::Eq(to_string(eq))),
            (None, Some(ne)) => Ok(Condition::Ne(to_string(ne))),
            (Some(_), Some(_)) => Err(D::Error::custom("mutually exclusive fields")),
            (None, None) => Err(D::Error::custom(
                "missing condition, please give 'expect_eq' or 'expect_ne'",
            )),
        }
    }
}
