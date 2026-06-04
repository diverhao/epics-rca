use ::log::{debug, info, warn};
use std::collections::HashMap;
use std::env;
use std::fmt;

/**
 * Type of an environment variable.
 *
 * Some types may not be used.
 */
#[derive(Debug, Clone)]
pub enum EnvType {
    String(String),
    StringArray(Vec<String>),
    Double(f64),
    DoubleArray(Vec<f64>),
    Integer(i32),
    IntegerArray(Vec<i32>),
    Boolean(bool),
    BooleanArray(Vec<bool>),
}

#[derive(Debug, Clone)]
pub enum EnvSource {
    Default,
    Os,
    User,
}

pub struct Env {
    // EPICS default env
    default: HashMap<String, EnvType>,
    // Operating system defined env
    os: HashMap<String, EnvType>,
    // User defined env
    user: HashMap<String, EnvType>,
}

/// Prints a table: one row per variable, columns for each source.
///
/// Column order: variable_name | default | os | user
impl fmt::Display for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::collections::BTreeSet;

        // Collect all unique variable names (sorted for consistent output)
        let mut all_names: BTreeSet<&String> = BTreeSet::new();
        all_names.extend(self.default.keys());
        all_names.extend(self.os.keys());
        all_names.extend(self.user.keys());

        if all_names.is_empty() {
            return write!(f, "(no environment variables set)");
        }

        // Helper to format an EnvValue for display
        fn fmt_val(v: &EnvType) -> String {
            match v {
                EnvType::String(s) => format!("\"{s}\""),
                EnvType::StringArray(arr) => format!("[{}]", arr.join(", ")),
                EnvType::Integer(n) => format!("{n}"),
                EnvType::IntegerArray(arr) => {
                    let nums: Vec<String> = arr.iter().map(|n| n.to_string()).collect();
                    format!("[{}]", nums.join(", "))
                }
                EnvType::Double(n) => format!("{n}"),
                EnvType::DoubleArray(arr) => {
                    let nums: Vec<String> = arr.iter().map(|n| n.to_string()).collect();
                    format!("[{}]", nums.join(", "))
                }
                EnvType::Boolean(b) => format!("{b}"),
                EnvType::BooleanArray(arr) => {
                    let bools: Vec<String> = arr.iter().map(|n| n.to_string()).collect();
                    format!("[{}]", bools.join(", "))
                }
            }
        }

        // Determine column widths
        let name_width = all_names.iter().map(|n| n.len()).max().unwrap_or(4).max(7);
        let default_width = self
            .default
            .values()
            .map(|v| fmt_val(v).len())
            .max()
            .unwrap_or(7)
            .max(7);
        let os_width = self
            .os
            .values()
            .map(|v| fmt_val(v).len())
            .max()
            .unwrap_or(2)
            .max(2);
        let user_width = self
            .user
            .values()
            .map(|v| fmt_val(v).len())
            .max()
            .unwrap_or(4)
            .max(4);

        // Header
        writeln!(
            f,
            "┌{:─^name_width$}┬{:─^default_width$}┬{:─^os_width$}┬{:─^user_width$}┐",
            "",
            "",
            "",
            "",
            name_width = name_width + 2,
            default_width = default_width + 2,
            os_width = os_width + 2,
            user_width = user_width + 2
        )?;
        writeln!(
            f,
            "│ {:^name_width$} │ {:^default_width$} │ {:^os_width$} │ {:^user_width$} │",
            "VARIABLE",
            "DEFAULT",
            "OS",
            "USER",
            name_width = name_width,
            default_width = default_width,
            os_width = os_width,
            user_width = user_width
        )?;
        writeln!(
            f,
            "├{:─^name_width$}┼{:─^default_width$}┼{:─^os_width$}┼{:─^user_width$}┤",
            "",
            "",
            "",
            "",
            name_width = name_width + 2,
            default_width = default_width + 2,
            os_width = os_width + 2,
            user_width = user_width + 2
        )?;

        // Data rows
        for name in &all_names {
            let default_val = self
                .default
                .get(*name)
                .map(|v| fmt_val(v))
                .unwrap_or("—".to_string());
            let os_val = self
                .os
                .get(*name)
                .map(|v| fmt_val(v))
                .unwrap_or("—".to_string());
            let user_val = self
                .user
                .get(*name)
                .map(|v| fmt_val(v))
                .unwrap_or("—".to_string());

            writeln!(
                f,
                "│ {: <name_width$} │ {: <default_width$} │ {: <os_width$} │ {: <user_width$} │",
                name,
                default_val,
                os_val,
                user_val,
                name_width = name_width,
                default_width = default_width,
                os_width = os_width,
                user_width = user_width
            )?;
        }

        // Footer
        write!(
            f,
            "└{:─^name_width$}┴{:─^default_width$}┴{:─^os_width$}┴{:─^user_width$}┘",
            "",
            "",
            "",
            "",
            name_width = name_width + 2,
            default_width = default_width + 2,
            os_width = os_width + 2,
            user_width = user_width + 2
        )
    }
}

impl Env {
    pub fn new(user_env: Vec<(&str, &str)>) -> Self {
        let user_env: HashMap<&str, &str> = user_env.into_iter().collect();
        // create an empty struct
        let mut env = Env {
            default: HashMap::new(), // key-value types are inferred
            os: HashMap::new(),
            user: HashMap::new(),
        };
        env.load_default_env();
        env.read_os_env();
        env.read_user_env(user_env);
        info!("epics-rca is running with these settings\n{}", env);
        env
    }

    pub fn set(self: &mut Self, name: &str, value: EnvType, source: EnvSource) {
        let dest_map = match source {
            EnvSource::Default => &mut self.default,
            EnvSource::Os => &mut self.os,
            EnvSource::User => &mut self.user,
        };
        dest_map.insert(name.to_string(), value);
    }

    /**
     * Read default environment variables
     */
    fn load_default_env(self: &mut Self) {
        // Channel Access defaults
        self.set(
            "EPICS_CA_ADDR_LIST",
            EnvType::StringArray(vec!["127.0.0.1".to_string()]),
            EnvSource::Default,
        );
        self.set(
            "EPICS_CA_AUTO_ADDR_LIST",
            EnvType::String("YES".to_string()),
            EnvSource::Default,
        );
        self.set(
            "EPICS_CA_CONN_TMO",
            EnvType::Double(30.0),
            EnvSource::Default,
        );
        self.set(
            "EPICS_CA_REPEATER_PORT",
            EnvType::Integer(5065),
            EnvSource::Default,
        );
        self.set(
            "EPICS_CA_SERVER_PORT",
            EnvType::Integer(5064),
            EnvSource::Default,
        );
        self.set(
            "EPICS_CA_MAX_ARRAY_BYTES",
            EnvType::Integer(16384),
            EnvSource::Default,
        );
        self.set(
            "EPICS_CA_AUTO_ARRAY_BYTES",
            EnvType::Integer(16384),
            EnvSource::Default,
        );
        self.set(
            "EPICS_CA_MAX_SEARCH_PERIOD",
            EnvType::Double(300.0),
            EnvSource::Default,
        );
        self.set(
            "EPICS_CA_NAME_SERVERS",
            EnvType::String("".to_string()),
            EnvSource::Default,
        );
        self.set(
            "EPICS_CA_MCAST_TTL",
            EnvType::Integer(1),
            EnvSource::Default,
        );
        self.set(
            "EPICS_CA_BEACON_PERIOD",
            EnvType::Double(15.0),
            EnvSource::Default,
        );

        // PVAccess defaults
        self.set(
            "EPICS_PVA_ADDR_LIST",
            EnvType::StringArray(vec![String::new()]),
            EnvSource::Default,
        );
        self.set(
            "EPICS_PVA_AUTO_ADDR_LIST",
            EnvType::String("YES".to_string()),
            EnvSource::Default,
        );
        self.set(
            "EPICS_PVA_SERVER_PORT",
            EnvType::Integer(5075),
            EnvSource::Default,
        );
        self.set(
            "EPICS_PVA_BEACON_PERIOD",
            EnvType::Double(15.0),
            EnvSource::Default,
        );
        self.set(
            "EPICS_PVA_CONN_TMO",
            EnvType::Double(30.0),
            EnvSource::Default,
        );
        self.set(
            "EPICS_PVA_BROADCAST_PORT",
            EnvType::Integer(5076),
            EnvSource::Default,
        );
        self.set(
            "EPICS_PVA_NAME_SERVERS",
            EnvType::String("".to_string()),
            EnvSource::Default,
        );
        self.set(
            "EPICS_PVA_MAX_ARRAY_BYTES",
            EnvType::Integer(16384),
            EnvSource::Default,
        );
        self.set(
            "EPICS_PVA_SEARCH_MAX_INTERVAL",
            EnvType::Double(300.0),
            EnvSource::Default,
        );
        self.set(
            "EPICS_PVA_PROVIDER_NAMES",
            EnvType::String("".to_string()),
            EnvSource::Default,
        );
    }

    fn read_os_env(self: &mut Self) {
        debug!("----- parsing OS env ------");
        for (name, _default_value) in &self.default.clone() {
            if let Ok(os_value_raw) = env::var(name) {
                debug!("Try to parse \"{}\" as EPICS env {}", os_value_raw, name);
                self.parse_value(name, &os_value_raw, EnvSource::Os);
            } else {
                debug!("OS does not define {}", name);
            }
        }
    }

    fn read_user_env(self: &mut Self, user_env: HashMap<&str, &str>) {
        debug!("----- parsing user env ------");
        for (name, _default_value) in &self.default.clone() {
            if let Some(user_value_raw) = user_env.get(name.as_str()) {
                debug!("Try to parse \"{}\" as EPICS env {}", user_value_raw, name);
                self.parse_value(name, &user_value_raw, EnvSource::User);
            } else {
                debug!("User does not provide {}", name);
            }
        }
    }

    fn print_parse_value_error(self: &Self, raw: &str, name: &str) {
        warn!("Failed to parse \"{}\" as {}", raw, name);
    }

    /// Parse an OS environment string into the appropriate EnvType,
    /// guided by the default value's type.
    fn parse_value(self: &mut Self, name: &str, raw: &str, source: EnvSource) {
        let Some(default_value) = self.default.get(name) else {
            // No default entry for this variable — nothing to parse against
            return;
        };

        match default_value {
            EnvType::String(_) => {
                let string_value = raw.trim().to_string();
                self.set(name, EnvType::String(string_value), source);
            }
            EnvType::StringArray(_) => {
                let string_array_value: Vec<String> =
                    raw.split_whitespace().map(|s| s.to_string()).collect();
                if string_array_value.len() > 0 {
                    self.set(name, EnvType::StringArray(string_array_value), source);
                } else {
                    self.print_parse_value_error(raw, name);
                }
            }
            EnvType::Integer(_) => {
                if let Ok(integer_value) = raw.parse::<i32>() {
                    self.set(name, EnvType::Integer(integer_value), source);
                } else {
                    self.print_parse_value_error(raw, name);
                }
            }
            EnvType::IntegerArray(_) => {
                let integer_array_value: Vec<i32> = raw
                    .split_whitespace()
                    .filter_map(|s| s.parse::<i32>().ok())
                    .collect();
                if integer_array_value.len() > 0 {
                    self.set(name, EnvType::IntegerArray(integer_array_value), source);
                } else {
                    self.print_parse_value_error(raw, name);
                }
            }
            EnvType::Double(_) => {
                if let Ok(double_value) = raw.parse::<f64>() {
                    self.set(name, EnvType::Double(double_value), source);
                } else {
                    self.print_parse_value_error(raw, name);
                }
            }
            EnvType::DoubleArray(_) => {
                let double_array_value: Vec<f64> = raw
                    .split_whitespace()
                    .filter_map(|s| s.parse::<f64>().ok())
                    .collect();
                if double_array_value.len() > 0 {
                    self.set(name, EnvType::DoubleArray(double_array_value), source);
                } else {
                    self.print_parse_value_error(raw, name);
                }
            }
            EnvType::Boolean(_) => {
                if let Ok(boolean_value) = raw.parse::<bool>() {
                    self.set(name, EnvType::Boolean(boolean_value), source);
                } else {
                    self.print_parse_value_error(raw, name);
                }
            }
            EnvType::BooleanArray(_) => {
                let boolean_array_value: Vec<bool> = raw
                    .split_whitespace()
                    .filter_map(|s| s.parse::<bool>().ok())
                    .collect();
                if boolean_array_value.len() > 0 {
                    self.set(name, EnvType::BooleanArray(boolean_array_value), source);
                } else {
                    self.print_parse_value_error(raw, name);
                }
            }
        };
    }

    fn get_default_env(self: &Self, name: &str) -> Option<&EnvType> {
        self.default.get(name)
    }

    fn get_user_env(self: &Self, name: &str) -> Option<&EnvType> {
        self.user.get(name)
    }
    fn get_os_env(self: &Self, name: &str) -> Option<&EnvType> {
        self.os.get(name)
    }

    pub fn get_env(self: &Self, name: &str) -> Option<&EnvType> {
        if let Some(value) = self.get_user_env(name) {
            Some(value)
        } else if let Some(value) = self.get_os_env(name) {
            Some(value)
        } else if let Some(value) = self.get_default_env(name) {
            Some(value)
        } else {
            None
        }
    }

    /**
     * Returns which source provides the value for the given variable name.
     *
     * Priority: User > OS > Default.
     *
     * Returns `None` if the variable is not defined in any source.
     */
    pub fn get_env_source(self: &Self, name: &str) -> Option<EnvSource> {
        if let Some(_value) = self.get_user_env(name) {
            Some(EnvSource::User)
        } else if let Some(_value) = self.get_os_env(name) {
            Some(EnvSource::Os)
        } else if let Some(_value) = self.get_default_env(name) {
            Some(EnvSource::Default)
        } else {
            None
        }
    }
}
