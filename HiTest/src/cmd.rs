use super::condition::Condition;
use libparser::LibParse;
use log::{debug, error, info};
use serde::Deserialize;
use std::error::Error;

#[derive(Debug, Deserialize, Clone)]
pub struct Cmd {
    pub opfunc: String,
    #[serde(flatten)]
    pub condition: Condition,
    pub args: Vec<String>,
    #[serde(default)]
    pub perf: bool,
}

impl Cmd {
    fn parse_value(s: &str) -> Result<i64, Box<dyn Error>> {
        let actual_s = if s.starts_with('!') { &s[1..] } else { s };
        if actual_s.starts_with("0x") || actual_s.starts_with("0X") {
            i64::from_str_radix(&actual_s[2..], 16).map_err(|e| {
                error!("Failed to parse hexadecimal value: {}", s);
                e.into()
            })
        } else {
            actual_s.parse::<i64>().map_err(|e| {
                error!("Failed to parse decimal value: {}", s);
                e.into()
            })
        }
    }

    pub fn run(&self) -> Result<bool, Box<dyn Error>> {
        let lib_parser = LibParse::get_instance()?.read().unwrap();
        let ret: i64 = if self.perf {
            let (ans, perf) = lib_parser.execute_with_perf(self.opfunc.clone(), &self.args)?;
            info!("cmd '{}' executed cost {}", self.opfunc, perf);
            ans
        } else {
            lib_parser.execute(self.opfunc.clone(), &self.args)?
        };

        let (expected, operator, is_success) = match &self.condition {
            Condition::Eq(v) => {
                let expected = Cmd::parse_value(&v)?;
                if v.starts_with("!") {
                    (expected, "!=", ret != expected)
                } else {
                    (expected, "==", ret == expected)
                }
            }
            Condition::Ne(v) => {
                let expected = Cmd::parse_value(&v)?;
                (expected, "!=", ret != expected)
            }
        };

        let message = format!(
            "execute cmd: {}{:?}, expect return value {}{}, actual: {}",
            self.opfunc, self.args, operator, expected, ret
        );

        if !is_success {
            error!("{}", message);
        } else {
            debug!("{} succeeded", message);
        }

        Ok(is_success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_creation() {
        let cmd_str1 = r#"
            opfunc = "Call_malloc"
            expect_eq = 0
            args = ["len=$alloc_size", "mem_idx=1"]
        "#;

        let cmd_str2 = r#"
            opfunc = "Call_malloc"
            expect_eq = 0
            perf = true
            args = ["len=$alloc_size", "mem_idx=1"]
        "#;

        let cmd_str3 = r#"
            opfunc = "Call_malloc"
            expect_ne = 0
            perf = true
            args = ["len=$alloc_size", "mem_idx=1"]
        "#;

        let cmd1: Cmd = toml::from_str(cmd_str1).unwrap();
        assert_eq!(cmd1.opfunc, "Call_malloc");
        assert!(matches!(cmd1.condition, Condition::Eq(_)));
        assert_eq!(cmd1.args.len(), 2);
        assert!(!cmd1.perf);

        let cmd2: Cmd = toml::from_str(cmd_str2).unwrap();
        assert!(cmd2.perf);

        let cmd3: Cmd = toml::from_str(cmd_str3).unwrap();
        assert!(matches!(cmd3.condition, Condition::Ne(_)));
    }
}
