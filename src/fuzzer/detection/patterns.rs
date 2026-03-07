#![allow(dead_code)]

pub fn get_detection_patterns() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "sql_syntax",
            "SQL syntax|mysql_fetch|ORA-|PLS-|Unclosed quotation",
        ),
        (
            "stack_trace",
            "at java.|at org.|Traceback|PHP Fatal|Stack trace",
        ),
        ("file_path", "/etc/passwd|/var/log|C:\\\\Windows|/var/www"),
        ("credentials", "password|api_key|secret|token|private_key"),
        ("errors", "Fatal error|Exception|Warning:|Error:"),
        ("database", "mysql_|pg_|sqlite_|PDO|SQLSTATE"),
        ("config", "\\.env|config\\.php|wp-config|database\\.yml"),
        ("aws", "aws_access_key|AWS_ACCESS|AKIA|aws_secret"),
        ("keys", "BEGIN.*PRIVATE KEY|ghp_|gho_|xox[baprs]"),
        (
            "connections",
            "jdbc:|mysql://|postgres://|mongodb://|redis://",
        ),
    ]
}

pub fn get_database_error_patterns() -> Vec<&'static str> {
    vec![
        "SQL syntax",
        "mysql_fetch",
        "ORA-",
        "PLS-",
        "Unclosed quotation mark",
        "quoted string not properly terminated",
        "You have an error in your SQL syntax",
        "Warning: mysql_",
        "PostgreSQL query failed",
        "pg_query()",
        "SQLSTATE[",
        "PDO::SQLSTATE",
        "Microsoft OLE DB Provider",
        "ODBC Microsoft Access Driver",
        "Syntax error in string in query expression",
        "Data type mismatch in criteria expression",
        "supplied argument is not a valid MySQL",
        "valid MySQL result",
        "on MySQL result index",
    ]
}

pub fn get_stack_trace_patterns() -> Vec<&'static str> {
    vec![
        "at java.",
        "at org.",
        "at com.",
        "at net.",
        "Traceback (most recent call last)",
        "File \"/",
        "line ",
        "PHP Fatal error",
        "PHP Warning",
        "PHP Notice",
        "PHP Stack trace",
        "#0 ",
        "Stack trace:",
        "at System.",
        "at Microsoft.",
        "   at ",
        "Error: ",
        "Exception: ",
        "Uncaught exception",
        "NullPointerException",
        "IndexOutOfRangeException",
        "TypeError",
        "ValueError",
        "KeyError",
        "AttributeError",
    ]
}

pub fn get_file_leak_patterns() -> Vec<&'static str> {
    vec![
        "/etc/passwd",
        "/etc/shadow",
        "/etc/hosts",
        "/var/log/",
        "/home/",
        "/root/",
        "/usr/local/",
        "C:\\Windows",
        "C:\\Users",
        "C:\\inetpub",
        "/var/www/",
        "/app/",
        ".env",
        "config.php",
        "wp-config.php",
        "database.yml",
        "settings.py",
        "application.properties",
        "web.config",
        "php.ini",
        "my.cnf",
        "postgresql.conf",
        "redis.conf",
    ]
}

pub fn get_credential_patterns() -> Vec<&'static str> {
    vec![
        "password",
        "passwd",
        "api_key",
        "apikey",
        "api-key",
        "secret_key",
        "secretkey",
        "secret-key",
        "access_token",
        "accesstoken",
        "access-token",
        "refresh_token",
        "auth_token",
        "authtoken",
        "private_key",
        "privatekey",
        "private-key",
        "aws_access_key_id",
        "aws_secret_access_key",
        "AWS_ACCESS_KEY",
        "AWS_SECRET_KEY",
        "connection string",
        "connectionString",
    ]
}

pub fn get_key_patterns() -> Vec<&'static str> {
    vec![
        "-----BEGIN RSA PRIVATE KEY-----",
        "-----BEGIN PRIVATE KEY-----",
        "-----BEGIN OPENSSH PRIVATE KEY-----",
        "-----BEGIN EC PRIVATE KEY-----",
        "-----BEGIN PGP PRIVATE KEY BLOCK-----",
        "AKIA[0-9A-Z]{16}",
        "xox[baprs]-[0-9]{10,13}",
        "ghp_[a-zA-Z0-9]{36}",
        "gho_[a-zA-Z0-9]{36}",
        "ghu_[a-zA-Z0-9]{36}",
        "ghs_[a-zA-Z0-9]{36}",
        "ghr_[a-zA-Z0-9]{36}",
        "sk-[a-zA-Z0-9]{20}T3BlbkFJ",
        "sk-[a-zA-Z0-9]{32}",
        "AIza[0-9A-Za-z_-]{35}",
        "ya29\\.[0-9A-Za-z_-]+",
        "sk_live_[0-9a-zA-Z]{24}",
        "sk_test_[0-9a-zA-Z]{24}",
        "rk_live_[0-9a-zA-Z]{24}",
        "rk_test_[0-9a-zA-Z]{24}",
    ]
}

pub fn get_connection_string_patterns() -> Vec<&'static str> {
    vec![
        "jdbc:",
        "mysql://",
        "postgres://",
        "postgresql://",
        "mongodb://",
        "mongodb+srv://",
        "redis://",
        "amqp://",
        "smtp://",
        "ldap://",
        "ldaps://",
        "ftp://",
        "sftp://",
    ]
}

pub fn get_debug_patterns() -> Vec<&'static str> {
    vec![
        "DEBUG=true",
        "debug=true",
        "debug=1",
        "debug_mode",
        "enable_debug",
        "APP_DEBUG",
        "DEBUG_MODE",
        "phpinfo()",
        "var_dump",
        "print_r",
        "console.log",
        "console.debug",
        "console.error",
        "debugger;",
        "breakpoint",
        "pdb.set_trace",
        "binding.pry",
        "debugpy",
        "django-debug-toolbar",
        "Laravel Debugbar",
        "Whoops!",
        "Symfony Profiler",
    ]
}
