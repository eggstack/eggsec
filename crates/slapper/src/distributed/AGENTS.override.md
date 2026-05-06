# Distributed Module Override

Specialized guidance for the distributed computing module.

## TLS

`distributed/io.rs` has `StreamWrapper` enum:
- `Plain` - Unencrypted stream
- `TlsClient` - TLS client connection
- `TlsServer` - TLS server connection

## TlsServer

`TlsServer::from_pem(cert_path, key_path)` loads PEM cert + key files.

## TlsClient

`TlsClient::new(domain)` creates client with `NoVerifier` (insecure, for internal use).

## Worker Registration Protocol

Workers use `RemoteClient` to register with the coordinator via TCP (not HTTP):

```rust
let client = RemoteClient::new_plaintext(psk);
client.register_worker(host, port, worker_id, hostname, capabilities).await?;
```

The coordinator expects line-based JSON messages via `CommandMessage::Register` and `CommandMessage::Heartbeat`, not HTTP POST requests.

## URL Parsing

Worker coordinator URLs should be `host:port` format (no http:// prefix):
```rust
fn parse_coordinator_url(url: &str) -> Result<(&str, u16)> {
    // Strip http:// or https://, parse host:port
}
```