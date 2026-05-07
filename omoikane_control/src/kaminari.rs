use crate::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KaminariDeviceProfile {
    pub name: String,
    pub host: String,
    pub username: String,
    pub netconf_port: u16,
    pub read_only: bool,
    pub commit_confirm_timeout_minutes: u16,
}

impl KaminariDeviceProfile {
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

    pub fn validate(&self) -> Result<(), KaminariControlError> {
        validate_non_empty("name", &self.name)?;
        validate_non_empty("host", &self.host)?;
        validate_non_empty("username", &self.username)?;
        if self.netconf_port == 0 {
            return Err(KaminariControlError::InvalidPort("netconf_port"));
        }
        if self.commit_confirm_timeout_minutes == 0 {
            return Err(KaminariControlError::InvalidTimeout(
                "commit_confirm_timeout_minutes",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KaminariOperation {
    SystemFacts,
    InterfaceTerse,
    RouteSummary,
    CommitCheck,
    CommitConfirmed,
    RollbackZero,
}

impl KaminariOperation {
    pub const fn tool_name(self) -> &'static str {
        match self {
            Self::SystemFacts => "kaminari.system_facts",
            Self::InterfaceTerse => "kaminari.interface_terse",
            Self::RouteSummary => "kaminari.route_summary",
            Self::CommitCheck => "kaminari.commit_check",
            Self::CommitConfirmed => "kaminari.commit_confirmed",
            Self::RollbackZero => "kaminari.rollback_zero",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::SystemFacts => "Le hostname, modelo, versao e fatos de chassis.",
            Self::InterfaceTerse => "Le estado resumido das interfaces para saude do rack.",
            Self::RouteSummary => "Le resumo de rotas antes de expor os servicos Omoikane.",
            Self::CommitCheck => "Valida a configuracao candidata sem aplicar commit.",
            Self::CommitConfirmed => "Aplica commit com janela automatica de rollback.",
            Self::RollbackZero => "Retorna ao checkpoint ativo de configuracao.",
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
pub struct KaminariMcpTool {
    pub name: String,
    pub description: String,
    pub operation: KaminariOperation,
    pub requires_write: bool,
}

impl KaminariMcpTool {
    pub fn from_operation(operation: KaminariOperation) -> Self {
        Self {
            name: operation.tool_name().to_string(),
            description: operation.description().to_string(),
            operation,
            requires_write: operation.requires_write(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KaminariMcpCatalog {
    pub device: KaminariDeviceProfile,
    pub tools: Vec<KaminariMcpTool>,
}

impl KaminariMcpCatalog {
    pub fn rack_default(device: KaminariDeviceProfile) -> Self {
        Self {
            device,
            tools: vec![
                KaminariMcpTool::from_operation(KaminariOperation::SystemFacts),
                KaminariMcpTool::from_operation(KaminariOperation::InterfaceTerse),
                KaminariMcpTool::from_operation(KaminariOperation::RouteSummary),
                KaminariMcpTool::from_operation(KaminariOperation::CommitCheck),
                KaminariMcpTool::from_operation(KaminariOperation::CommitConfirmed),
                KaminariMcpTool::from_operation(KaminariOperation::RollbackZero),
            ],
        }
    }

    pub fn rpc_for_tool(&self, name: &str) -> Result<String, KaminariControlError> {
        self.device.validate()?;
        let tool = self
            .tools
            .iter()
            .find(|tool| tool.name == name)
            .ok_or_else(|| KaminariControlError::UnknownTool(name.to_string()))?;
        if self.device.read_only && tool.requires_write {
            return Err(KaminariControlError::WriteBlocked(tool.name.clone()));
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
pub enum KaminariControlError {
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

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), KaminariControlError> {
    if value.trim().is_empty() {
        Err(KaminariControlError::EmptyField(field))
    } else {
        Ok(())
    }
}

impl std::fmt::Display for KaminariControlError {
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

impl std::error::Error for KaminariControlError {}

#[cfg(test)]
mod tests {
    use super::{KaminariControlError, KaminariDeviceProfile, KaminariMcpCatalog};

    #[test]
    fn catalog_exposes_read_rpcs() {
        let catalog = KaminariMcpCatalog::rack_default(KaminariDeviceProfile::rack_default(
            "rack-core",
            "192.0.2.10",
            "netops",
        ));

        let rpc = catalog.rpc_for_tool("kaminari.interface_terse").unwrap();
        assert!(rpc.contains("<get-interface-information>"));
        assert!(rpc.contains("<terse/>"));
    }

    #[test]
    fn catalog_blocks_write_tools_when_read_only() {
        let catalog = KaminariMcpCatalog::rack_default(KaminariDeviceProfile::rack_default(
            "rack-core",
            "192.0.2.10",
            "netops",
        ));

        assert_eq!(
            catalog.rpc_for_tool("kaminari.commit_confirmed"),
            Err(KaminariControlError::WriteBlocked(
                "kaminari.commit_confirmed".to_string()
            ))
        );
    }

    #[test]
    fn catalog_allows_confirmed_commit_when_enabled() {
        let catalog = KaminariMcpCatalog::rack_default(
            KaminariDeviceProfile::rack_default("rack-core", "192.0.2.10", "netops").allow_write(),
        );

        let rpc = catalog.rpc_for_tool("kaminari.commit_confirmed").unwrap();
        assert!(rpc.contains("<confirmed/>"));
        assert!(rpc.contains("<confirm-timeout>5</confirm-timeout>"));
    }
}
