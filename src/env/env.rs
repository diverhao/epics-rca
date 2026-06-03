use std::collections::HashMap;
use std::fmt;
use std::env;

/**
 * Type of an environment variable.
 * 
 */
#[derive(Debug, Clone)]
pub enum EnvType {
    String(String),
    StringArray(Vec<String>),
    Number(f64),
    NumberArray(Vec<f64>),
    Boolean(bool),
}

pub enum EnvSource {
    Default,
    Os,
    User,
}

pub struct Env {
    default: HashMap<String, EnvType>,
    os: HashMap<String, EnvType>,
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
                EnvType::String(s)        => format!("\"{s}\""),
                EnvType::StringArray(arr) => format!("[{}]", arr.join(", ")),
                EnvType::Number(n)        => format!("{n}"),
                EnvType::NumberArray(arr) => {
                    let nums: Vec<String> = arr.iter().map(|n| n.to_string()).collect();
                    format!("[{}]", nums.join(", "))
                },
                EnvType::Boolean(b)       => format!("{b}"),
            }
        }

        // Determine column widths
        let name_width = all_names.iter().map(|n| n.len()).max().unwrap_or(4).max(7);
        let default_width = self.default.values()
            .map(|v| fmt_val(v).len())
            .max().unwrap_or(7).max(7);
        let os_width = self.os.values()
            .map(|v| fmt_val(v).len())
            .max().unwrap_or(2).max(2);
        let user_width = self.user.values()
            .map(|v| fmt_val(v).len())
            .max().unwrap_or(4).max(4);

        // Header
        writeln!(f, "┌{:─^name_width$}┬{:─^default_width$}┬{:─^os_width$}┬{:─^user_width$}┐",
            "", "", "", "",
            name_width = name_width + 2,
            default_width = default_width + 2,
            os_width = os_width + 2,
            user_width = user_width + 2)?;
        writeln!(f, "│ {:^name_width$} │ {:^default_width$} │ {:^os_width$} │ {:^user_width$} │",
            "VARIABLE", "DEFAULT", "OS", "USER",
            name_width = name_width,
            default_width = default_width,
            os_width = os_width,
            user_width = user_width)?;
        writeln!(f, "├{:─^name_width$}┼{:─^default_width$}┼{:─^os_width$}┼{:─^user_width$}┤",
            "", "", "", "",
            name_width = name_width + 2,
            default_width = default_width + 2,
            os_width = os_width + 2,
            user_width = user_width + 2)?;

        // Data rows
        for name in &all_names {
            let default_val = self.default.get(*name).map(|v| fmt_val(v)).unwrap_or("—".to_string());
            let os_val = self.os.get(*name).map(|v| fmt_val(v)).unwrap_or("—".to_string());
            let user_val = self.user.get(*name).map(|v| fmt_val(v)).unwrap_or("—".to_string());

            writeln!(f, "│ {: <name_width$} │ {: <default_width$} │ {: <os_width$} │ {: <user_width$} │",
                name,
                default_val,
                os_val,
                user_val,
                name_width = name_width,
                default_width = default_width,
                os_width = os_width,
                user_width = user_width)?;
        }

        // Footer
        write!(f, "└{:─^name_width$}┴{:─^default_width$}┴{:─^os_width$}┴{:─^user_width$}┘",
            "", "", "", "",
            name_width = name_width + 2,
            default_width = default_width + 2,
            os_width = os_width + 2,
            user_width = user_width + 2)
    }
}

impl Env {
    pub fn new(user_env: HashMap<String, EnvType>) -> Self {
        // create an empty struct
        let mut env = Env {
            default: HashMap::new(), // key-value types are inferred
            os: HashMap::new(),
            user: HashMap::new(),
        };
        // update the struct
        env.load_default_env();
        env.read_os_env();
        env.set_user_env(user_env);
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

    pub fn get(self: &Self, name: &str) {

    }

    /**
     * Read default environment variables
     */
    fn load_default_env(self: &mut Self) {
        // todo: all envs, including CA and PVA, only for client
        let epics_ca_addr_list = vec!["127.0.0.1".to_string()];
        self.set("EPICS_CA_ADDR_LIST", EnvType::StringArray(epics_ca_addr_list), EnvSource::Default);
        let epics_ca_auto_addr_list = true;
        self.set("EPICS_CA_AUTO_ADDR_LIST", EnvType::Boolean(epics_ca_auto_addr_list), EnvSource::Default);
    }

    fn read_os_env(self: &mut Self) {        
        for (name, default_value) in &self.default.clone() {
            if let Ok(os_value_raw) = env::var(name) {
                // let typed_val = Self::parse_os_value(&os_value, default_value);
                // self.set(name, typed_val, EnvSource::Os);
                self.parse_os_value(name, &os_value_raw, default_value);
            }
        }
    }

    /// Parse an OS environment string into the appropriate EnvType,
    /// guided by the default value's type.
    fn parse_os_value(self: &mut Self, name: &str, raw: &str, default_value: &EnvType) -> EnvType {
        match default_value {
            // EnvType::StringArray(_) => {
            //     let parts: Vec<String> = raw
            //         .split_whitespace()
            //         .map(|s| s.to_string())
            //         .collect();
            //     EnvType::StringArray(parts)
            // }
            // EnvType::Number(_) => {
            //     raw.parse::<f64>()
            //         .map(EnvType::Number)
            //         .unwrap_or_else(|_| EnvType::String(raw.to_string()))
            // }
            // EnvType::NumberArray(_) => {
            //     let parts: Vec<f64> = raw
            //         .split_whitespace()
            //         .filter_map(|s| s.parse::<f64>().ok())
            //         .collect();
            //     if parts.is_empty() {
            //         EnvType::String(raw.to_string())
            //     } else {
            //         EnvType::NumberArray(parts)
            //     }
            // }
            EnvType::Boolean(_) => {
                if let Ok(bool_value) = raw.parse::<bool>() {
                    self.set(name, EnvType::Boolean(bool_value), EnvSource::Os);
                }

                // raw.parse::<bool>()
                    // .map(EnvType::Boolean)
                    // .unwrap_or_else(|_| EnvType::String(raw.to_string()))
            }
            // Default is string or no default → keep as string
            _ => {},
        }
        EnvType::String(String::from("abc"))
    }

    fn set_user_env(self: &mut Self, user_env: HashMap<String, EnvType>) {
        self.user = user_env;
    }
}

