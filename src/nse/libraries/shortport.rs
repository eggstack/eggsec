//! NSE shortport library wrapper
//!
//! Functions for building short portrules.
//! Based on Nmap's shortport library: https://nmap.org/nsedoc/lib/shortport.html

use mlua::{Lua, Table, Value};

const WELL_KNOWN_SERVICES: &[(&str, &[u16])] = &[
    ("http", &[80, 8080, 8000, 8888, 3000, 5000]),
    ("https", &[443, 8443, 9443]),
    ("ftp", &[21, 20]),
    ("ssh", &[22]),
    ("telnet", &[23]),
    ("smtp", &[25, 587, 465]),
    ("pop3", &[110, 995]),
    ("imap", &[143, 993]),
    ("mysql", &[3306]),
    ("redis", &[6379]),
    ("mongodb", &[27017, 27018, 27019]),
    ("mssql", &[1433]),
    ("postgres", &[5432, 5433]),
    ("oracle", &[1521]),
    ("ldap", &[389, 636]),
    ("kerberos", &[88]),
    ("dns", &[53]),
    ("snmp", &[161, 162]),
    ("samba", &[139, 445]),
    ("rdp", &[3389]),
    ("vnc", &[5900, 5901]),
    ("git", &[9418]),
    ("svn", &[3690]),
    ("irc", &[6667, 6668, 6669]),
    ("ntp", &[123]),
    ("syslog", &[514]),
    ("rsh", &[514]),
    ("finger", &[79]),
    ("whois", &[43]),
];

fn parse_port_spec(value: &Value) -> Vec<u16> {
    let mut ports = Vec::new();

    match value {
        Value::Number(n) => {
            if let Some(n) = n.as_u64() {
                ports.push(n as u16);
            }
        }
        Value::String(s) => {
            let s = s.to_string();
            for part in s.split(',') {
                let part = part.trim();
                if let Some((start, end)) = part.split_once('-') {
                    let start: u16 = start.trim().parse().unwrap_or(0);
                    let end: u16 = end.trim().parse().unwrap_or(65535);
                    ports.extend(start..=end);
                } else if let Ok(port) = part.parse::<u16>() {
                    ports.push(port);
                }
            }
        }
        Value::Table(t) => {
            for i in 1..=t.len() {
                if let Ok(v) = t.get::<Value>(i) {
                    ports.extend(parse_port_spec(&v));
                }
            }
        }
        _ => {}
    }

    ports
}

fn parse_service_spec(value: &Value) -> Vec<String> {
    let mut services = Vec::new();

    match value {
        Value::String(s) => {
            let s = s.to_string();
            for part in s.split(',') {
                services.push(part.trim().to_lowercase());
            }
        }
        Value::Table(t) => {
            for i in 1..=t.len() {
                if let Ok(s) = t.get::<String>(i) {
                    services.push(s.to_lowercase());
                }
            }
        }
        _ => {}
    }

    services
}

fn get_ports_for_service(service: &str) -> Option<&'static [u16]> {
    for (name, ports) in WELL_KNOWN_SERVICES {
        if *name == service {
            return Some(ports);
        }
    }
    None
}

fn match_service_on_port(service: &str, port: u16) -> bool {
    get_ports_for_service(service)
        .map(|ports| ports.contains(&port))
        .unwrap_or(false)
}

fn check_nmap_ports<F>(
    lua: &Lua,
    ports: &[u16],
    proto: Option<&str>,
    state: Option<&str>,
    mut check: F,
) -> bool
where
    F: FnMut(u16, Option<&str>, Option<&str>, Option<&str>) -> bool,
{
    let nmap = match lua.globals().get::<Table>("nmap") {
        Ok(n) => n,
        Err(_) => return ports.is_empty(),
    };

    let port_table = match nmap.get::<Table>("ports") {
        Ok(t) => t,
        Err(_) => return ports.is_empty(),
    };

    for i in 1..=port_table.len() {
        if let Ok(p) = port_table.get::<Table>(i) {
            let port_num: Option<u16> = p.get("number").ok();
            let port_proto: Option<String> = p.get("protocol").ok();
            let port_state: Option<String> = p.get("state").ok();
            let port_service: Option<String> = p.get("service").ok();

            if let Some(num) = port_num {
                if !ports.is_empty() && !ports.contains(&num) {
                    continue;
                }
                if let Some(p) = proto {
                    if port_proto.as_ref().map(|s| s.as_str()) != Some(p) {
                        continue;
                    }
                }
                if let Some(s) = state {
                    if port_state.as_ref().map(|st| st.as_str()) != Some(s) {
                        continue;
                    }
                }

                if check(
                    num,
                    port_proto.as_deref(),
                    port_state.as_deref(),
                    port_service.as_deref(),
                ) {
                    return true;
                }
            }
        }
    }

    false
}

pub fn register_shortport_library(lua: &Lua) {
    let globals = lua.globals();
    let shortport = lua
        .create_table()
        .expect("Failed to create shortport table");

    shortport
        .set(
            "portnumber",
            lua.create_function(
                |lua, (ports, protos, states): (Value, Option<Value>, Option<Value>)| {
                    let requested = parse_port_spec(&ports);
                    let proto = protos.and_then(|p| p.as_str().map(String::from));
                    let state = states.and_then(|s| s.as_str().map(String::from));

                    let found = check_nmap_ports(
                        lua,
                        &requested,
                        proto.as_deref(),
                        state.as_deref(),
                        |_, _, _, _| true,
                    );

                    Ok(found || requested.is_empty())
                },
            ),
        )
        .ok();

    shortport
        .set(
            "service",
            lua.create_function(
                |lua, (services, protos, states): (Value, Option<Value>, Option<Value>)| {
                    let requested = parse_service_spec(&services);
                    let proto = protos.and_then(|p| p.as_str().map(String::from));
                    let state = states.and_then(|s| s.as_str().map(String::from));

                    let found = check_nmap_ports(
                        lua,
                        &[],
                        proto.as_deref(),
                        state.as_deref(),
                        |_, _, _, svc| {
                            if let Some(s) = svc {
                                let s_lower = s.to_lowercase();
                                for req in &requested {
                                    if s_lower.contains(req) || req.contains(&s_lower) {
                                        return true;
                                    }
                                }
                            }
                            false
                        },
                    );

                    if found {
                        return Ok(true);
                    }

                    for req in &requested {
                        let found_by_port = check_nmap_ports(
                            lua,
                            &[],
                            proto.as_deref(),
                            state.as_deref(),
                            |port, _, _, _| match_service_on_port(req, port),
                        );
                        if found_by_port {
                            return Ok(true);
                        }
                    }

                    Ok(requested.is_empty())
                },
            ),
        )
        .ok();

    shortport
        .set(
            "ssl",
            lua.create_function(|lua, (ports, protos): (Value, Option<Value>)| {
                let requested = parse_port_spec(&ports);
                let proto = protos.and_then(|p| p.as_str().map(String::from));

                let ssl_ports = [443, 8443, 993, 995, 465, 636];

                let found = check_nmap_ports(
                    lua,
                    &requested,
                    proto.as_deref(),
                    None,
                    |port, _, _, svc| {
                        if ssl_ports.contains(&port) {
                            return true;
                        }
                        if let Some(s) = svc {
                            let s_lower = s.to_lowercase();
                            return s_lower.contains("ssl")
                                || s_lower.contains("tls")
                                || s_lower.contains("https");
                        }
                        false
                    },
                );

                Ok(found)
            }),
        )
        .ok();

    shortport
        .set(
            "http",
            lua.create_function(|lua, (ports, protos): (Value, Option<Value>)| {
                let requested = parse_port_spec(&ports);
                let proto = protos.and_then(|p| p.as_str().map(String::from));
                let http_ports = [80, 8080, 8000, 8888, 3000, 5000, 443];

                let found = check_nmap_ports(
                    lua,
                    &requested,
                    proto.as_deref(),
                    None,
                    |port, _, _, svc| {
                        if http_ports.contains(&port) {
                            return true;
                        }
                        if let Some(s) = svc {
                            let s_lower = s.to_lowercase();
                            return s_lower.contains("http") && !s_lower.contains("nothttp");
                        }
                        false
                    },
                );

                Ok(found)
            }),
        )
        .ok();

    shortport
        .set(
            "ftp",
            lua.create_function(|lua, (ports, protos): (Value, Option<Value>)| {
                let requested = parse_port_spec(&ports);
                let proto = protos.and_then(|p| p.as_str().map(String::from));

                let found = check_nmap_ports(
                    lua,
                    &requested,
                    proto.as_deref(),
                    None,
                    |port, _, _, svc| {
                        if port == 21 || port == 20 {
                            return true;
                        }
                        if let Some(s) = svc {
                            return s.to_lowercase().contains("ftp");
                        }
                        false
                    },
                );

                Ok(found)
            }),
        )
        .ok();

    shortport
        .set(
            "ssh",
            lua.create_function(|lua, (ports, protos): (Value, Option<Value>)| {
                let requested = parse_port_spec(&ports);
                let proto = protos.and_then(|p| p.as_str().map(String::from));

                let found = check_nmap_ports(
                    lua,
                    &requested,
                    proto.as_deref(),
                    None,
                    |port, _, _, svc| {
                        if port == 22 {
                            return true;
                        }
                        if let Some(s) = svc {
                            return s.to_lowercase().contains("ssh");
                        }
                        false
                    },
                );

                Ok(found)
            }),
        )
        .ok();

    shortport
        .set(
            "smtp",
            lua.create_function(|lua, (ports, protos): (Value, Option<Value>)| {
                let requested = parse_port_spec(&ports);
                let proto = protos.and_then(|p| p.as_str().map(String::from));
                let smtp_ports = [25, 587, 465];

                let found = check_nmap_ports(
                    lua,
                    &requested,
                    proto.as_deref(),
                    None,
                    |port, _, _, svc| {
                        if smtp_ports.contains(&port) {
                            return true;
                        }
                        if let Some(s) = svc {
                            return s.to_lowercase().contains("smtp")
                                || s.to_lowercase().contains("mail");
                        }
                        false
                    },
                );

                Ok(found)
            }),
        )
        .ok();

    shortport
        .set(
            "pop3",
            lua.create_function(|lua, (ports, protos): (Value, Option<Value>)| {
                let requested = parse_port_spec(&ports);
                let proto = protos.and_then(|p| p.as_str().map(String::from));

                let found = check_nmap_ports(
                    lua,
                    &requested,
                    proto.as_deref(),
                    None,
                    |port, _, _, svc| {
                        if port == 110 || port == 995 {
                            return true;
                        }
                        if let Some(s) = svc {
                            return s.to_lowercase().contains("pop3");
                        }
                        false
                    },
                );

                Ok(found)
            }),
        )
        .ok();

    shortport
        .set(
            "imap",
            lua.create_function(|lua, (ports, protos): (Value, Option<Value>)| {
                let requested = parse_port_spec(&ports);
                let proto = protos.and_then(|p| p.as_str().map(String::from));

                let found = check_nmap_ports(
                    lua,
                    &requested,
                    proto.as_deref(),
                    None,
                    |port, _, _, svc| {
                        if port == 143 || port == 993 {
                            return true;
                        }
                        if let Some(s) = svc {
                            return s.to_lowercase().contains("imap");
                        }
                        false
                    },
                );

                Ok(found)
            }),
        )
        .ok();

    shortport
        .set(
            "mysql",
            lua.create_function(|lua, (ports, protos): (Value, Option<Value>)| {
                let requested = parse_port_spec(&ports);
                let proto = protos.and_then(|p| p.as_str().map(String::from));

                let found = check_nmap_ports(
                    lua,
                    &requested,
                    proto.as_deref(),
                    None,
                    |port, _, _, svc| {
                        if port == 3306 {
                            return true;
                        }
                        if let Some(s) = svc {
                            return s.to_lowercase().contains("mysql");
                        }
                        false
                    },
                );

                Ok(found)
            }),
        )
        .ok();

    shortport
        .set(
            "redis",
            lua.create_function(|lua, (ports, protos): (Value, Option<Value>)| {
                let requested = parse_port_spec(&ports);
                let proto = protos.and_then(|p| p.as_str().map(String::from));

                let found = check_nmap_ports(
                    lua,
                    &requested,
                    proto.as_deref(),
                    None,
                    |port, _, _, svc| {
                        if port == 6379 {
                            return true;
                        }
                        if let Some(s) = svc {
                            return s.to_lowercase().contains("redis");
                        }
                        false
                    },
                );

                Ok(found)
            }),
        )
        .ok();

    shortport
        .set(
            "mongodb",
            lua.create_function(|lua, (ports, protos): (Value, Option<Value>)| {
                let requested = parse_port_spec(&ports);
                let proto = protos.and_then(|p| p.as_str().map(String::from));
                let mongo_ports = [27017, 27018, 27019];

                let found = check_nmap_ports(
                    lua,
                    &requested,
                    proto.as_deref(),
                    None,
                    |port, _, _, svc| {
                        if mongo_ports.contains(&port) {
                            return true;
                        }
                        if let Some(s) = svc {
                            let lower = s.to_lowercase();
                            return lower.contains("mongodb") || lower.contains("mongod");
                        }
                        false
                    },
                );

                Ok(found)
            }),
        )
        .ok();

    shortport
        .set(
            "mssql",
            lua.create_function(|lua, (ports, protos): (Value, Option<Value>)| {
                let requested = parse_port_spec(&ports);
                let proto = protos.and_then(|p| p.as_str().map(String::from));

                let found = check_nmap_ports(
                    lua,
                    &requested,
                    proto.as_deref(),
                    None,
                    |port, _, _, svc| {
                        if port == 1433 {
                            return true;
                        }
                        if let Some(s) = svc {
                            let lower = s.to_lowercase();
                            return lower.contains("mssql") || lower.contains("microsoft-ss");
                        }
                        false
                    },
                );

                Ok(found)
            }),
        )
        .ok();

    shortport
        .set(
            "port_or_service",
            lua.create_function(
                |lua, (ports, services, states): (Value, Value, Option<Value>)| {
                    let port_fn = lua
                        .globals()
                        .get::<mlua::Function>("shortport")
                        .ok()
                        .and_then(|sp| sp.get::<mlua::Function>("portnumber").ok());
                    let svc_fn = lua
                        .globals()
                        .get::<mlua::Function>("shortport")
                        .ok()
                        .and_then(|sp| sp.get::<mlua::Function>("service").ok());

                    if let Some(pf) = port_fn {
                        if pf
                            .call::<_, bool>((ports.clone(), None::<Value>, None::<Value>))
                            .unwrap_or(false)
                        {
                            return Ok(true);
                        }
                    }

                    if let Some(sf) = svc_fn {
                        if sf
                            .call::<_, bool>((services, None::<Value>, states))
                            .unwrap_or(false)
                        {
                            return Ok(true);
                        }
                    }

                    Ok(false)
                },
            ),
        )
        .ok();

    shortport
        .set(
            "port_range",
            lua.create_function(|_lua, range: Value| {
                let ports = parse_port_spec(&range);
                Ok(!ports.is_empty())
            }),
        )
        .ok();

    shortport
        .set(
            "port_is_excluded",
            lua.create_function(|_lua, port: Value| {
                let excluded = [0, 1, 2, 9, 6000, 6665, 6666, 6667, 6668, 6669];
                match &port {
                    Value::Number(n) => {
                        if let Some(p) = n.as_u64() {
                            return Ok(excluded.contains(&(p as u16)));
                        }
                    }
                    Value::String(s) => {
                        if let Ok(p) = s.to_string().parse::<u16>() {
                            return Ok(excluded.contains(&p));
                        }
                    }
                    _ => {}
                }
                Ok(false)
            }),
        )
        .ok();

    shortport
        .set(
            "any",
            lua.create_function(|_lua, _: (Value, Value)| Ok(true)),
        )
        .ok();

    shortport
        .set(
            "tcp",
            lua.create_function(|lua, (ports, states): (Value, Option<Value>)| {
                let requested = parse_port_spec(&ports);
                let state = states.and_then(|s| s.as_str().map(String::from));

                let found = check_nmap_ports(
                    lua,
                    &requested,
                    Some("tcp"),
                    state.as_deref(),
                    |_, _, _, _| true,
                );

                Ok(found)
            }),
        )
        .ok();

    shortport
        .set(
            "udp",
            lua.create_function(|lua, (ports, states): (Value, Option<Value>)| {
                let requested = parse_port_spec(&ports);
                let state = states.and_then(|s| s.as_str().map(String::from));

                let found = check_nmap_ports(
                    lua,
                    &requested,
                    Some("udp"),
                    state.as_deref(),
                    |_, _, _, _| true,
                );

                Ok(found)
            }),
        )
        .ok();

    shortport
        .set(
            "why",
            lua.create_function(|lua, _: Value| {
                let result = lua.create_table()?;
                result.set("reason", "no match")?;
                result.set("matched", false)?;
                Ok(result)
            }),
        )
        .ok();

    shortport
        .set("version", lua.create_function(|_lua, _: ()| Ok("1.0.0")))
        .ok();

    globals.set("shortport", shortport).ok();
}
