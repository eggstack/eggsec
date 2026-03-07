//! NSE sslcert library wrapper
//!
//! Provides SSL/TLS certificate parsing and validation.
//! Based on Nmap's sslcert library: https://nmap.org/nsedoc/lib/sslcert.html

use mlua::Lua;
use native_tls::TlsConnector;
use openssl::asn1::Asn1TimeRef;
use openssl::base64::decode_block;
use openssl::x509::{X509Name, X509};
use std::net::TcpStream;

fn parse_x509_name(name: &X509Name) -> String {
    name.entries()
        .map(|e| {
            let key = e.object().nid().short_name().unwrap_or("Unknown");
            let value = e
                .data()
                .as_utf8()
                .map(|s| s.to_string())
                .unwrap_or_default();
            format!("{}={}", key, value)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn asn1_time_to_unix(time: &Asn1TimeRef) -> Option<i64> {
    time.to_string().ok().and_then(|s| {
        chrono::NaiveDateTime::parse_from_str(&s, "%b %e %H:%M:%S %Y %z")
            .ok()
            .or_else(|| chrono::NaiveDateTime::parse_from_str(&s, "%b %e %H:%M:%S %Y").ok())
            .map(|dt| dt.and_utc().timestamp())
    })
}

pub fn register_sslcert_library(lua: &Lua) {
    let globals = lua.globals();
    let sslcert = lua.create_table().expect("Failed to create sslcert table");

    sslcert
        .set(
            "get_certificate",
            lua.create_function(|lua, (host, port): (String, u16)| {
                let result = lua.create_table().expect("Failed to create result table");

                let connector = match TlsConnector::builder()
                    .danger_accept_invalid_certs(true)
                    .danger_accept_invalid_hostnames(true)
                    .build()
                {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = result.set("error", format!("TLS connector error: {}", e));
                        return Ok(result);
                    }
                };

                let stream = match TcpStream::connect(format!("{}:{}", host, port)) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = result.set("error", format!("Connection error: {}", e));
                        return Ok(result);
                    }
                };

                let tls_stream = match connector.connect(&host, stream) {
                    Ok(t) => t,
                    Err(e) => {
                        let _ = result.set("error", format!("TLS handshake error: {}", e));
                        return Ok(result);
                    }
                };

                let cert = match tls_stream.peer_certificate() {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = result.set("error", format!("Certificate error: {}", e));
                        return Ok(result);
                    }
                };

                if let Some(certificate) = cert {
                    let cert_der = match certificate.to_der() {
                        Ok(d) => d,
                        Err(e) => {
                            let _ = result.set("error", format!("DER encoding error: {}", e));
                            return Ok(result);
                        }
                    };

                    let cert_pem = openssl::base64::encode_block(&cert_der);
                    let pem_str = format!(
                        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
                        cert_pem
                    );
                    let _ = result.set("pem", pem_str);
                    let _ = result.set("der", cert_der);

                    if let Ok(x509) = X509::from_der(&cert_der) {
                        if let Some(subject) = x509.subject_name().as_str().ok() {
                            let _ = result.set("subject", subject);
                        }
                        if let Some(issuer) = x509.issuer_name().as_str().ok() {
                            let _ = result.set("issuer", issuer);
                        }

                        if let Ok(not_before) = x509.not_before().to_string() {
                            let _ = result.set("notBefore", not_before);
                        }
                        if let Ok(not_after) = x509.not_after().to_string() {
                            let _ = result.set("notAfter", not_after);
                        }

                        if let Ok(serial) = x509.serial_number().to_bn() {
                            if let Ok(serial_hex) = serial.to_hex_str() {
                                let _ = result.set("serialNumber", serial_hex.to_string());
                            }
                        }

                        let _ = result.set("version", x509.version() as i32 + 1);

                        if let Ok(fingerprint) = x509.digest(openssl::hash::MessageDigest::sha1()) {
                            let fp_hex = fingerprint
                                .iter()
                                .map(|b| format!("{:02x}", b))
                                .collect::<String>();
                            let _ = result.set("fingerprint", fp_hex);
                        }

                        if let Ok(fingerprint256) =
                            x509.digest(openssl::hash::MessageDigest::sha256())
                        {
                            let fp_hex = fingerprint256
                                .iter()
                                .map(|b| format!("{:02x}", b))
                                .collect::<String>();
                            let _ = result.set("fingerprint256", fp_hex);
                        }

                        let _ = result.set(
                            "sig_alg",
                            x509.signature_algorithm()
                                .object()
                                .nid()
                                .short_name()
                                .unwrap_or("Unknown"),
                        );

                        if let Ok(pkey) = x509.public_key() {
                            if let Ok(pkey_type) = pkey.id() {
                                let _ = result.set("key_type", format!("{:?}", pkey_type));
                            }
                        }
                    }
                }

                Ok(result)
            }),
        )
        .ok();

    sslcert
        .set(
            "parse_certificate",
            lua.create_function(|lua, cert_pem: String| {
                let result = lua.create_table().expect("Failed to create result table");

                let pem_clean = cert_pem
                    .replace("-----BEGIN CERTIFICATE-----", "")
                    .replace("-----END CERTIFICATE-----", "")
                    .replace('\n', "");

                let cert_der = match decode_block(&pem_clean) {
                    Ok(d) => d,
                    Err(e) => {
                        let _ = result.set("error", format!("Base64 decode error: {}", e));
                        return Ok(result);
                    }
                };

                let _ = result.set("der", cert_der.clone());

                if let Ok(x509) = X509::from_der(&cert_der) {
                    if let Some(subject) = x509.subject_name().as_str().ok() {
                        let _ = result.set("subject", subject);
                    }
                    if let Some(issuer) = x509.issuer_name().as_str().ok() {
                        let _ = result.set("issuer", issuer);
                    }

                    if let Ok(not_before) = x509.not_before().to_string() {
                        let _ = result.set("notBefore", not_before);
                    }
                    if let Ok(not_after) = x509.not_after().to_string() {
                        let _ = result.set("notAfter", not_after);
                    }

                    if let Ok(serial) = x509.serial_number().to_bn() {
                        if let Ok(serial_hex) = serial.to_hex_str() {
                            let _ = result.set("serialNumber", serial_hex.to_string());
                        }
                    }

                    let _ = result.set("version", x509.version() as i32 + 1);

                    if let Ok(fingerprint) = x509.digest(openssl::hash::MessageDigest::sha1()) {
                        let fp_hex = fingerprint
                            .iter()
                            .map(|b| format!("{:02x}", b))
                            .collect::<String>();
                        let _ = result.set("fingerprint", fp_hex);
                    }

                    if let Ok(fingerprint256) = x509.digest(openssl::hash::MessageDigest::sha256())
                    {
                        let fp_hex = fingerprint256
                            .iter()
                            .map(|b| format!("{:02x}", b))
                            .collect::<String>();
                        let _ = result.set("fingerprint256", fp_hex);
                    }

                    let _ = result.set(
                        "sig_alg",
                        x509.signature_algorithm()
                            .object()
                            .nid()
                            .short_name()
                            .unwrap_or("Unknown"),
                    );

                    if let Ok(pkey) = x509.public_key() {
                        if let Ok(pkey_type) = pkey.id() {
                            let _ = result.set("key_type", format!("{:?}", pkey_type));
                        }
                        if let Ok(bits) = pkey.bits() {
                            let _ = result.set("key_bits", bits as i32);
                        }
                    }

                    let subject_alt_names = lua.create_table().ok();
                    if let Some(sans) = subject_alt_names {
                        if let Ok(ext) = x509.subject_alt_name() {
                            if let Ok(general_names) = ext.general_names() {
                                for (i, name) in general_names.iter().enumerate() {
                                    let _ = match name {
                                        openssl::x509::GeneralName::DNS(dns) => {
                                            sans.set(i + 1, dns.to_string())
                                        }
                                        openssl::x509::GeneralName::IP(ip) => {
                                            sans.set(i + 1, ip.to_string())
                                        }
                                        _ => sans.set(i + 1, "".to_string()),
                                    };
                                }
                            }
                        }
                        let _ = result.set("subjectAltName", sans);
                    }
                }

                Ok(result)
            }),
        )
        .ok();

    sslcert
        .set(
            "get_certificate_by_host",
            lua.create_function(|lua, (host, port): (String, u16)| {
                let get_cert = lua
                    .globals()
                    .get::<mlua::Table>("sslcert")
                    .and_then(|t| t.get::<mlua::Function>("get_certificate"));

                if let Ok(func) = get_cert {
                    return func.call((host, port));
                }
                let result = lua.create_table().expect("Failed to create result table");
                let _ = result.set("error", "get_certificate function not found");
                Ok(result)
            }),
        )
        .ok();

    sslcert
        .set(
            "verify",
            lua.create_function(|lua, (host, port): (String, u16)| {
                let result = lua.create_table().expect("Failed to create result table");

                let connector = match TlsConnector::builder()
                    .danger_accept_invalid_certs(false)
                    .danger_accept_invalid_hostnames(true)
                    .build()
                {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = result.set("valid", false);
                        let _ = result.set("error", format!("TLS connector error: {}", e));
                        return Ok(result);
                    }
                };

                let stream = match TcpStream::connect(format!("{}:{}", host, port)) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = result.set("valid", false);
                        let _ = result.set("error", format!("Connection error: {}", e));
                        return Ok(result);
                    }
                };

                match connector.connect(&host, stream) {
                    Ok(_tls_stream) => {
                        let _ = result.set("valid", true);
                    }
                    Err(e) => {
                        let _ = result.set("valid", false);
                        let _ = result.set("error", e.to_string());
                    }
                }

                Ok(result)
            }),
        )
        .ok();

    sslcert
        .set("version", lua.create_function(|_lua, _: ()| Ok("1.0.0")))
        .ok();

    globals.set("sslcert", sslcert).ok();
}
