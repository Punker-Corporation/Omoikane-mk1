use crate::json;
use std::collections::BTreeSet;
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkRobotDevice {
    pub name: String,
    pub role: String,
    pub management_address: String,
    pub platform: String,
}

impl NetworkRobotDevice {
    pub fn new(
        name: impl Into<String>,
        role: impl Into<String>,
        management_address: impl Into<String>,
        platform: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            role: role.into(),
            management_address: management_address.into(),
            platform: platform.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkRobotCheck {
    pub name: String,
    pub device: String,
    pub operation: String,
    pub expected_fragment: String,
    pub timeout_seconds: u16,
    pub critical: bool,
}

impl NetworkRobotCheck {
    pub fn new(
        name: impl Into<String>,
        device: impl Into<String>,
        operation: impl Into<String>,
        expected_fragment: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            device: device.into(),
            operation: operation.into(),
            expected_fragment: expected_fragment.into(),
            timeout_seconds: 30,
            critical: true,
        }
    }

    pub fn non_critical(mut self) -> Self {
        self.critical = false;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkRobotPlan {
    pub name: String,
    pub devices: Vec<NetworkRobotDevice>,
    pub checks: Vec<NetworkRobotCheck>,
    pub rollback_steps: Vec<String>,
}

impl NetworkRobotPlan {
    pub fn rack_acceptance(
        server_overlay_ip: impl Into<String>,
        junos_host: impl Into<String>,
    ) -> Self {
        let server_overlay_ip = server_overlay_ip.into();
        Self {
            name: "omoikane-rack-acceptance".to_string(),
            devices: vec![
                NetworkRobotDevice::new(
                    "omoikane-server",
                    "runtime",
                    server_overlay_ip,
                    "omoikane-rust",
                ),
                NetworkRobotDevice::new("rack-core", "router", junos_host, "junos-netconf"),
            ],
            checks: vec![
                NetworkRobotCheck::new(
                    "server-status",
                    "omoikane-server",
                    "GET /status",
                    "\"state\":\"running\"",
                ),
                NetworkRobotCheck::new(
                    "router-interface-terse",
                    "rack-core",
                    "junos.interface_terse",
                    "<interface-information",
                ),
                NetworkRobotCheck::new(
                    "router-route-summary",
                    "rack-core",
                    "junos.route_summary",
                    "<route-summary-information",
                )
                .non_critical(),
            ],
            rollback_steps: vec![
                "stop omoikane server listener".to_string(),
                "remove temporary overlay peer if it was applied".to_string(),
                "restore previous rack firewall profile if acceptance fails".to_string(),
            ],
        }
    }

    pub fn validate(&self) -> Result<(), NetworkRobotPlanError> {
        validate_non_empty("name", &self.name)?;

        let mut device_names = BTreeSet::new();
        for device in &self.devices {
            validate_non_empty("device.name", &device.name)?;
            validate_non_empty("device.role", &device.role)?;
            validate_non_empty("device.management_address", &device.management_address)?;
            validate_non_empty("device.platform", &device.platform)?;
            if !device_names.insert(device.name.clone()) {
                return Err(NetworkRobotPlanError::DuplicateDevice(device.name.clone()));
            }
        }

        let mut check_names = BTreeSet::new();
        for check in &self.checks {
            validate_non_empty("check.name", &check.name)?;
            validate_non_empty("check.device", &check.device)?;
            validate_non_empty("check.operation", &check.operation)?;
            validate_non_empty("check.expected_fragment", &check.expected_fragment)?;
            if check.timeout_seconds == 0 {
                return Err(NetworkRobotPlanError::InvalidTimeout(check.name.clone()));
            }
            if !device_names.contains(&check.device) {
                return Err(NetworkRobotPlanError::UnknownDevice(check.device.clone()));
            }
            if !check_names.insert(check.name.clone()) {
                return Err(NetworkRobotPlanError::DuplicateCheck(check.name.clone()));
            }
        }

        Ok(())
    }

    pub fn render_runbook(&self) -> Result<String, NetworkRobotPlanError> {
        self.validate()?;

        let mut runbook = String::new();
        writeln!(runbook, "Omoikane network robot plan: {}", self.name)
            .expect("writing runbook to String cannot fail");
        writeln!(runbook).expect("writing runbook to String cannot fail");
        writeln!(runbook, "devices:").expect("writing runbook to String cannot fail");
        for device in &self.devices {
            writeln!(
                runbook,
                "- {} [{}] {} via {}",
                device.name, device.role, device.management_address, device.platform
            )
            .expect("writing runbook to String cannot fail");
        }
        writeln!(runbook).expect("writing runbook to String cannot fail");
        writeln!(runbook, "checks:").expect("writing runbook to String cannot fail");
        for check in &self.checks {
            writeln!(
                runbook,
                "- {} on {}: {} expects {} ({}s, critical={})",
                check.name,
                check.device,
                check.operation,
                check.expected_fragment,
                check.timeout_seconds,
                check.critical
            )
            .expect("writing runbook to String cannot fail");
        }
        writeln!(runbook).expect("writing runbook to String cannot fail");
        writeln!(runbook, "rollback:").expect("writing runbook to String cannot fail");
        for step in &self.rollback_steps {
            writeln!(runbook, "- {step}").expect("writing runbook to String cannot fail");
        }
        Ok(runbook)
    }

    pub fn write_json(&self, out: &mut String) {
        out.push('{');
        json::push_string_field(out, "name", &self.name, true);
        json::push_field_name(out, "devices", false);
        out.push('[');
        for (index, device) in self.devices.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push('{');
            json::push_string_field(out, "name", &device.name, true);
            json::push_string_field(out, "role", &device.role, false);
            json::push_string_field(out, "management_address", &device.management_address, false);
            json::push_string_field(out, "platform", &device.platform, false);
            out.push('}');
        }
        out.push(']');
        json::push_field_name(out, "checks", false);
        out.push('[');
        for (index, check) in self.checks.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push('{');
            json::push_string_field(out, "name", &check.name, true);
            json::push_string_field(out, "device", &check.device, false);
            json::push_string_field(out, "operation", &check.operation, false);
            json::push_string_field(out, "expected_fragment", &check.expected_fragment, false);
            json::push_u16_field(out, "timeout_seconds", check.timeout_seconds, false);
            json::push_bool_field(out, "critical", check.critical, false);
            out.push('}');
        }
        out.push(']');
        json::push_field_name(out, "rollback_steps", false);
        out.push('[');
        for (index, step) in self.rollback_steps.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            json::push_string(out, step);
        }
        out.push(']');
        out.push('}');
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkRobotPlanError {
    EmptyField(&'static str),
    DuplicateDevice(String),
    DuplicateCheck(String),
    UnknownDevice(String),
    InvalidTimeout(String),
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), NetworkRobotPlanError> {
    if value.trim().is_empty() {
        Err(NetworkRobotPlanError::EmptyField(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{NetworkRobotDevice, NetworkRobotPlan, NetworkRobotPlanError};

    #[test]
    fn rack_acceptance_plan_renders_runbook() {
        let plan = NetworkRobotPlan::rack_acceptance("100.104.1.7", "192.0.2.1");
        let runbook = plan.render_runbook().unwrap();

        assert!(runbook.contains("omoikane-rack-acceptance"));
        assert!(runbook.contains("server-status"));
        assert!(runbook.contains("router-interface-terse"));
    }

    #[test]
    fn plan_rejects_duplicate_devices() {
        let mut plan = NetworkRobotPlan::rack_acceptance("100.104.1.7", "192.0.2.1");
        plan.devices.push(NetworkRobotDevice::new(
            "rack-core",
            "router",
            "192.0.2.2",
            "junos-netconf",
        ));

        assert_eq!(
            plan.validate(),
            Err(NetworkRobotPlanError::DuplicateDevice(
                "rack-core".to_string()
            ))
        );
    }
}
