use crate::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmoikaneDnsChoice {
    pub id: String,
    pub label: String,
    pub host: String,
    pub record_type: String,
    pub provider_hint: String,
    pub ttl_seconds: u16,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmoikaneDnsPlan {
    pub selected_host: String,
    pub bind_host: String,
    pub port: u16,
    pub refresh_seconds: u16,
    pub choices: Vec<OmoikaneDnsChoice>,
}

impl OmoikaneDnsPlan {
    pub fn new(
        bind_host: impl Into<String>,
        public_host: impl Into<String>,
        port: u16,
        requested_dns: Option<&str>,
    ) -> Self {
        let bind_host = bind_host.into();
        let public_host = public_host.into();
        let selected_host = requested_dns
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(public_host.as_str())
            .to_string();

        let mut choices = Vec::new();
        push_unique_choice(
            &mut choices,
            OmoikaneDnsChoice::new(
                "direct",
                "IP direto",
                &public_host,
                "A/AAAA",
                "auto_dns direto",
                30,
            ),
        );
        push_unique_choice(
            &mut choices,
            OmoikaneDnsChoice::new(
                "localhost",
                "Host local",
                host_for_local_bind(&bind_host),
                "A",
                "dns-updater local",
                15,
            ),
        );
        push_unique_choice(
            &mut choices,
            OmoikaneDnsChoice::new(
                "omoikane-localhost",
                "Omoikane localhost",
                "omoikane.localhost",
                "CNAME",
                "resolvedor local",
                15,
            ),
        );
        if let Some(requested_dns) = requested_dns
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            push_unique_choice(
                &mut choices,
                OmoikaneDnsChoice::new(
                    "custom",
                    "DNS escolhido",
                    requested_dns,
                    "CNAME/A",
                    "perfil dinamico",
                    30,
                ),
            );
        }

        for choice in &mut choices {
            choice.selected = choice.host.eq_ignore_ascii_case(&selected_host);
        }

        Self {
            selected_host,
            bind_host,
            port,
            refresh_seconds: 30,
            choices,
        }
    }

    pub fn access_url(&self, path: &str) -> String {
        let normalized = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };
        format!("http://{}:{}{}", self.selected_host, self.port, normalized)
    }

    pub fn write_json(&self, out: &mut String) {
        out.push('{');
        json::push_string_field(out, "selected_host", &self.selected_host, true);
        json::push_string_field(out, "bind_host", &self.bind_host, false);
        json::push_u16_field(out, "port", self.port, false);
        json::push_u16_field(out, "refresh_seconds", self.refresh_seconds, false);
        json::push_string_field(out, "console_url", &self.access_url("/"), false);
        json::push_string_field(out, "status_url", &self.access_url("/status"), false);
        json::push_field_name(out, "choices", false);
        out.push('[');
        for (index, choice) in self.choices.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            choice.write_json(out);
        }
        out.push(']');
        out.push('}');
    }
}

impl OmoikaneDnsChoice {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        host: impl Into<String>,
        record_type: impl Into<String>,
        provider_hint: impl Into<String>,
        ttl_seconds: u16,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            host: host.into(),
            record_type: record_type.into(),
            provider_hint: provider_hint.into(),
            ttl_seconds,
            selected: false,
        }
    }

    pub fn write_json(&self, out: &mut String) {
        out.push('{');
        json::push_string_field(out, "id", &self.id, true);
        json::push_string_field(out, "label", &self.label, false);
        json::push_string_field(out, "host", &self.host, false);
        json::push_string_field(out, "record_type", &self.record_type, false);
        json::push_string_field(out, "provider_hint", &self.provider_hint, false);
        json::push_u16_field(out, "ttl_seconds", self.ttl_seconds, false);
        json::push_bool_field(out, "selected", self.selected, false);
        out.push('}');
    }
}

fn push_unique_choice(choices: &mut Vec<OmoikaneDnsChoice>, choice: OmoikaneDnsChoice) {
    if choices
        .iter()
        .any(|existing| existing.host.eq_ignore_ascii_case(&choice.host))
    {
        return;
    }
    choices.push(choice);
}

fn host_for_local_bind(bind_host: &str) -> &str {
    match bind_host {
        "0.0.0.0" | "::" => "127.0.0.1",
        host => host,
    }
}

#[cfg(test)]
mod tests {
    use super::OmoikaneDnsPlan;

    #[test]
    fn dns_plan_selects_custom_host_and_keeps_local_choices() {
        let plan = OmoikaneDnsPlan::new("0.0.0.0", "100.104.1.20", 8080, Some("rack.example"));

        assert_eq!(plan.selected_host, "rack.example");
        assert!(plan.choices.iter().any(|choice| choice.host == "127.0.0.1"));
        assert!(
            plan.choices
                .iter()
                .any(|choice| choice.host == "rack.example")
        );
        assert_eq!(
            plan.access_url("/status"),
            "http://rack.example:8080/status"
        );
    }
}
