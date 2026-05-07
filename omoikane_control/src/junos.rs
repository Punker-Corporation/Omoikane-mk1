use crate::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JunosDeviceProfile {
    pub name: String,
    pub host: String,
    pub username: String,
    pub netconf_port: u16,
    pub read_only: bool,
    pub commit_confirm_timeout_minutes: u16,
}

impl JunosDeviceProfile {
    pub fn rack_default(
        name: impl Into<String>,
        host: impl Into<String>,
        username: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            host: host.into(),
            username: username.into(),
            netconf_port: 830,
            read_only: true,
            commit_confirm_timeout_minutes: 5,
        }
    }

    pub fn allow_write(mut self) -> Self {
        self.read_only = false;
        self
    }

    pub fn with_netconf_port(mut self, port: u16) -> Self {
        self.netconf_port = port;
        self
    }

    pub fn validate(&self) -> Result<(), JunosControlError> {
        validate_non_empty("name", &self.name)?;
        validate_non_empty("host", &self.host)?;
        validate_non_empty("username", &self.username)?;
        if self.netconf_port == 0 {
            return Err(JunosControlError::InvalidPort("netconf_port"));
        }
        if self.commit_confirm_timeout_minutes == 0 {
            return Err(JunosControlError::InvalidTimeout(
                "commit_confirm_timeout_minutes",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JunosOperation {
    SystemFacts,
    InterfaceTerse,
    RouteSummary,
    CommitCheck,
    CommitConfirmed,
    RollbackZero,
}

impl JunosOperation {
    pub const fn tool_name(self) -> &'static str {
        match self {
            Self::SystemFacts => "junos.system_facts",
            Self::InterfaceTerse => "junos.interface_terse",
            Self::RouteSummary => "junos.route_summary",
            Self::CommitCheck => "junos.commit_check",
            Self::CommitConfirmed => "junos.commit_confirmed",
            Self::RollbackZero => "junos.rollback_zero",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::SystemFacts => "Read hostname, model, version and chassis facts.",
            Self::InterfaceTerse => "Read terse interface state for rack health checks.",
            Self::RouteSummary => "Read route summary state before exposing Omoikane services.",
            Self::CommitCheck => "Validate a candidate configuration without committing it.",
            Self::CommitConfirmed => "Commit with an automatic rollback window.",
            Self::RollbackZero => "Rollback to the active configuration checkpoint.",
        }
    }

    pub const fn requires_write(self) -> bool {
        matches!(
            self,
            Self::CommitCheck | Self::CommitConfirmed | Self::RollbackZero
        )
    }

    fn rpc_xml(self, commit_confirm_timeout_minutes: u16) -> String {
        match self {
            Self::SystemFacts => rpc("get-system-information"),
            Self::InterfaceTerse => {
                "<rpc><get-interface-information><terse/></get-interface-information></rpc>"
                    .to_string()
            }
            Self::RouteSummary => rpc("get-route-summary-information"),
            Self::CommitCheck => {
                "<rpc><commit-configuration><check/></commit-configuration></rpc>".to_string()
            }
            Self::CommitConfirmed => format!(
                "<rpc><commit-configuration><confirmed/><confirm-timeout>{commit_confirm_timeout_minutes}</confirm-timeout></commit-configuration></rpc>"
            ),
            Self::RollbackZero => "<rpc><load-configuration rollback=\"0\"/></rpc>".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JunosMcpTool {
    pub name: String,
    pub description: String,
    pub operation: JunosOperation,
    pub requires_write: bool,
}

impl JunosMcpTool {
    pub fn from_operation(operation: JunosOperation) -> Self {
        Self {
            name: operation.tool_name().to_string(),
            description: operation.description().to_string(),
            operation,
            requires_write: operation.requires_write(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JunosMcpCatalog {
    pub device: JunosDeviceProfile,
    pub tools: Vec<JunosMcpTool>,
}

impl JunosMcpCatalog {
    pub fn rack_default(device: JunosDeviceProfile) -> Self {
        Self {
            device,
            tools: vec![
                JunosMcpTool::from_operation(JunosOperation::SystemFacts),
                JunosMcpTool::from_operation(JunosOperation::InterfaceTerse),
                JunosMcpTool::from_operation(JunosOperation::RouteSummary),
                JunosMcpTool::from_operation(JunosOperation::CommitCheck),
                JunosMcpTool::from_operation(JunosOperation::CommitConfirmed),
                JunosMcpTool::from_operation(JunosOperation::RollbackZero),
            ],
        }
    }

    pub fn rpc_for_tool(&self, name: &str) -> Result<String, JunosControlError> {
        self.device.validate()?;
        let tool = self
            .tools
            .iter()
            .find(|tool| tool.name == name)
            .ok_or_else(|| JunosControlError::UnknownTool(name.to_string()))?;
        if self.device.read_only && tool.requires_write {
            return Err(JunosControlError::WriteBlocked(tool.name.clone()));
        }
        Ok(tool
            .operation
            .rpc_xml(self.device.commit_confirm_timeout_minutes))
    }

    pub fn write_json(&self, out: &mut String) {
        out.push('{');
        json::push_field_name(out, "device", true);
        out.push('{');
        json::push_string_field(out, "name", &self.device.name, true);
        json::push_string_field(out, "host", &self.device.host, false);
        json::push_string_field(out, "username", &self.device.username, false);
        json::push_u16_field(out, "netconf_port", self.device.netconf_port, false);
        json::push_bool_field(out, "read_only", self.device.read_only, false);
        json::push_u16_field(
            out,
            "commit_confirm_timeout_minutes",
            self.device.commit_confirm_timeout_minutes,
            false,
        );
        out.push('}');
        json::push_field_name(out, "tools", false);
        out.push('[');
        for (index, tool) in self.tools.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push('{');
            json::push_string_field(out, "name", &tool.name, true);
            json::push_string_field(out, "description", &tool.description, false);
            json::push_bool_field(out, "requires_write", tool.requires_write, false);
            out.push('}');
        }
        out.push(']');
        out.push('}');
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JunosControlError {
    EmptyField(&'static str),
    InvalidPort(&'static str),
    InvalidTimeout(&'static str),
    UnknownTool(String),
    WriteBlocked(String),
}

fn rpc(name: &str) -> String {
    format!("<rpc><{}/></rpc>", xml_name(name))
}

fn xml_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            out.push(ch);
        }
    }
    out
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), JunosControlError> {
    if value.trim().is_empty() {
        Err(JunosControlError::EmptyField(field))
    } else {
        Ok(())
    }
}

impl std::fmt::Display for JunosControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField(field) => write!(f, "empty field: {field}"),
            Self::InvalidPort(field) => write!(f, "invalid port: {field}"),
            Self::InvalidTimeout(field) => write!(f, "invalid timeout: {field}"),
            Self::UnknownTool(tool) => write!(f, "unknown tool: {tool}"),
            Self::WriteBlocked(tool) => write!(f, "write operation blocked: {tool}"),
        }
    }
}

impl std::error::Error for JunosControlError {}

#[cfg(test)]
mod tests {
    use super::{JunosControlError, JunosDeviceProfile, JunosMcpCatalog};

    #[test]
    fn catalog_exposes_read_rpcs() {
        let catalog = JunosMcpCatalog::rack_default(JunosDeviceProfile::rack_default(
            "rack-core",
            "192.0.2.10",
            "netops",
        ));

        let rpc = catalog.rpc_for_tool("junos.interface_terse").unwrap();
        assert!(rpc.contains("<get-interface-information>"));
        assert!(rpc.contains("<terse/>"));
    }

    #[test]
    fn catalog_blocks_write_tools_when_read_only() {
        let catalog = JunosMcpCatalog::rack_default(JunosDeviceProfile::rack_default(
            "rack-core",
            "192.0.2.10",
            "netops",
        ));

        assert_eq!(
            catalog.rpc_for_tool("junos.commit_confirmed"),
            Err(JunosControlError::WriteBlocked(
                "junos.commit_confirmed".to_string()
            ))
        );
    }

    #[test]
    fn catalog_allows_confirmed_commit_when_enabled() {
        let catalog = JunosMcpCatalog::rack_default(
            JunosDeviceProfile::rack_default("rack-core", "192.0.2.10", "netops").allow_write(),
        );

        let rpc = catalog.rpc_for_tool("junos.commit_confirmed").unwrap();
        assert!(rpc.contains("<confirmed/>"));
        assert!(rpc.contains("<confirm-timeout>5</confirm-timeout>"));
    }
}
