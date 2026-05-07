use std::fmt::Write as _;

pub(crate) fn push_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                write!(out, "\\u{:04x}", ch as u32)
                    .expect("writing JSON escape to String cannot fail");
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

pub(crate) fn push_string_field(out: &mut String, name: &str, value: &str, first: bool) {
    push_field_name(out, name, first);
    push_string(out, value);
}

pub(crate) fn push_u16_field(out: &mut String, name: &str, value: u16, first: bool) {
    push_field_name(out, name, first);
    write!(out, "{value}").expect("writing JSON number to String cannot fail");
}

pub(crate) fn push_usize_field(out: &mut String, name: &str, value: usize, first: bool) {
    push_field_name(out, name, first);
    write!(out, "{value}").expect("writing JSON number to String cannot fail");
}

pub(crate) fn push_bool_field(out: &mut String, name: &str, value: bool, first: bool) {
    push_field_name(out, name, first);
    out.push_str(if value { "true" } else { "false" });
}

pub(crate) fn push_field_name(out: &mut String, name: &str, first: bool) {
    if !first {
        out.push(',');
    }
    push_string(out, name);
    out.push(':');
}
