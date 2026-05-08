use crate::json;
use std::collections::BTreeSet;
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MamoriDevice {
    pub name: String,
    pub role: String,
    pub management_address: String,
    pub platform: String,
}

impl MamoriDevice {
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
pub struct MamoriCheck {
    pub name: String,
    pub device: String,
    pub operation: String,
    pub expected_fragment: String,
    pub timeout_seconds: u16,
    pub critical: bool,
}

impl MamoriCheck {
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
pub struct MamoriPlan {
    pub name: String,
    pub devices: Vec<MamoriDevice>,
    pub checks: Vec<MamoriCheck>,
    pub rollback_steps: Vec<String>,
}

impl MamoriPlan {
    pub fn rack_acceptance(
        server_overlay_ip: impl Into<String>,
        kaminari_host: impl Into<String>,
    ) -> Self {
        let server_overlay_ip = server_overlay_ip.into();
        Self {
            name: "omoikane-rack-acceptance".to_string(),
            devices: vec![
                MamoriDevice::new(
                    "omoikane-server",
                    "runtime",
                    server_overlay_ip,
                    "omoikane-rust",
                ),
                MamoriDevice::new("rack-core", "router", kaminari_host, "kaminari-netconf"),
            ],
            checks: vec![
                MamoriCheck::new(
                    "server-status",
                    "omoikane-server",
                    "GET /status",
                    "\"state\":\"running\"",
                ),
                MamoriCheck::new(
                    "router-interface-terse",
                    "rack-core",
                    "kaminari.interface_terse",
                    "<interface-information",
                ),
                MamoriCheck::new(
                    "router-route-summary",
                    "rack-core",
                    "kaminari.route_summary",
                    "<route-summary-information",
                )
                .non_critical(),
            ],
            rollback_steps: vec![
                "parar o listener do servidor Omoikane".to_string(),
                "remover o peer overlay temporario caso tenha sido aplicado".to_string(),
                "restaurar o perfil anterior de firewall do rack se a aceitacao falhar".to_string(),
            ],
        }
    }

    pub fn validate(&self) -> Result<(), MamoriPlanError> {
        validate_non_empty("name", &self.name)?;

        let mut device_names = BTreeSet::new();
        for device in &self.devices {
            validate_non_empty("device.name", &device.name)?;
            validate_non_empty("device.role", &device.role)?;
            validate_non_empty("device.management_address", &device.management_address)?;
            validate_non_empty("device.platform", &device.platform)?;
            if !device_names.insert(device.name.clone()) {
                return Err(MamoriPlanError::DuplicateDevice(device.name.clone()));
            }
        }

        let mut check_names = BTreeSet::new();
        for check in &self.checks {
            validate_non_empty("check.name", &check.name)?;
            validate_non_empty("check.device", &check.device)?;
            validate_non_empty("check.operation", &check.operation)?;
            validate_non_empty("check.expected_fragment", &check.expected_fragment)?;
            if check.timeout_seconds == 0 {
                return Err(MamoriPlanError::InvalidTimeout(check.name.clone()));
            }
            if !device_names.contains(&check.device) {
                return Err(MamoriPlanError::UnknownDevice(check.device.clone()));
            }
            if !check_names.insert(check.name.clone()) {
                return Err(MamoriPlanError::DuplicateCheck(check.name.clone()));
            }
        }

        Ok(())
    }

    pub fn render_runbook(&self) -> Result<String, MamoriPlanError> {
        self.validate()?;

        let mut runbook = String::new();
        writeln!(runbook, "Plano Mamori da Omoikane: {}", self.name)
            .expect("writing runbook to String cannot fail");
        writeln!(runbook).expect("writing runbook to String cannot fail");
        writeln!(runbook, "dispositivos:").expect("writing runbook to String cannot fail");
        for device in &self.devices {
            writeln!(
                runbook,
                "- {} [{}] {} via {}",
                device.name, device.role, device.management_address, device.platform
            )
            .expect("writing runbook to String cannot fail");
        }
        writeln!(runbook).expect("writing runbook to String cannot fail");
        writeln!(runbook, "checagens:").expect("writing runbook to String cannot fail");
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
pub enum MamoriPlanError {
    EmptyField(&'static str),
    DuplicateDevice(String),
    DuplicateCheck(String),
    UnknownDevice(String),
    InvalidTimeout(String),
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), MamoriPlanError> {
    if value.trim().is_empty() {
        Err(MamoriPlanError::EmptyField(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{MamoriDevice, MamoriPlan, MamoriPlanError};

    #[test]
    fn rack_acceptance_plan_renders_runbook() {
        let plan = MamoriPlan::rack_acceptance("100.104.1.7", "192.0.2.1");
        let runbook = plan.render_runbook().unwrap();

        assert!(runbook.contains("omoikane-rack-acceptance"));
        assert!(runbook.contains("server-status"));
        assert!(runbook.contains("router-interface-terse"));
    }

    #[test]
    fn plan_rejects_duplicate_devices() {
        let mut plan = MamoriPlan::rack_acceptance("100.104.1.7", "192.0.2.1");
        plan.devices.push(MamoriDevice::new(
            "rack-core",
            "router",
            "192.0.2.2",
            "kaminari-netconf",
        ));

        assert_eq!(
            plan.validate(),
            Err(MamoriPlanError::DuplicateDevice("rack-core".to_string()))
        );
    }
}
