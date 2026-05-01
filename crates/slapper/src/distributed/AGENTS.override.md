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