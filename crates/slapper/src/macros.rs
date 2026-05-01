//! Macros for reducing code duplication and improving readability

#[macro_export]
macro_rules! run_if_enabled {
    ($condition:expr, $stage_name:expr, $stage:expr, $task:expr) => {{
        if $condition {
            $crate::recon::set_stage($stage, $stage_name);
            Some($task.await)
        } else {
            None
        }
    }};
}

#[macro_export]
macro_rules! stage_task {
    ($name:expr, $skip:expr, $stage:expr, $body:expr) => {{
        async {
            if $skip {
                None
            } else {
                $crate::recon::set_stage($stage, $name);
                Some($body.await)
            }
        }
    }};
}

#[macro_export]
macro_rules! recon_stage {
    ($skip:expr, $stage_name:literal, $stage:expr, $body:block) => {{
        async {
            if $skip {
                None
            } else {
                if let Ok(mut s) = $stage.lock() {
                    *s = $stage_name.to_string();
                }
                Some($body.await.ok().unwrap_or_default())
            }
        }
    }};
}

#[macro_export]
macro_rules! print_if_some {
    ($name:literal, $value:expr) => {{
        if let Some(ref v) = $value {
            println!("{}: {}", $name, v);
        }
    }};
}

#[macro_export]
macro_rules! option_as_result {
    ($expr:expr, $err_msg:literal) => {
        $expr.ok_or_else(|| anyhow::anyhow!($err_msg))?
    };
}

pub fn format_optional_field<T: std::fmt::Display>(s: &mut String, label: &str, value: &Option<T>) {
    if let Some(v) = value {
        s.push_str(&format!("{}: {}\n", label, v));
    }
}

pub fn format_list_field<T: std::fmt::Display>(
    s: &mut String,
    label: &str,
    values: &[T],
    prefix: &str,
) {
    if !values.is_empty() {
        s.push_str(&format!("{}\n", label));
        for v in values {
            s.push_str(&format!("\t{}{}\n", prefix, v));
        }
    }
}
