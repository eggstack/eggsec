"""Network programmability types for eggsec.

This submodule contains target resolution, transport sessions, protocol probes,
HTTP client, and WebSocket session types.

Maturity: provisional (Release 2 types)
"""

# Target resolution and basic probes (always available)
try:
    from .._core import (
        # Target resolution
        TargetPy as Target,
        ResolvedTargetPy as ResolvedTarget,
        ConnectionConfigPy as ConnectionConfig,
        TimeoutConfigPy as TimeoutConfig,
        RetryPolicyPy as RetryPolicy,
        SocketEndpointPy as SocketEndpoint,
        ConnectionTimingPy as ConnectionTiming,
        ConnectionMetadataPy as ConnectionMetadata,
        NetworkEvidencePy as NetworkEvidence,
        TranscriptEntryPy as TranscriptEntry,
        NetworkTranscriptPy as NetworkTranscript,
        resolve_target_sync,
        async_resolve_target,
        evidence_to_finding,
        # Transport (TCP/UDP)
        TcpConfigPy as TcpConfig,
        TcpSessionPy as TcpSession,
        TcpConnectResultPy as TcpConnectResult,
        TcpReadResultPy as TcpReadResult,
        TcpWriteResultPy as TcpWriteResult,
        UdpConfigPy as UdpConfig,
        UdpSocketPy as UdpSocket,
        UdpSendResultPy as UdpSendResult,
        UdpRecvResultPy as UdpRecvResult,
        UdpRecvFromResultPy as UdpRecvFromResult,
        BannerProbeResultPy as BannerProbeResult,
        AsyncTcpSessionPy as AsyncTcpSession,
        AsyncUdpSocketPy as AsyncUdpSocket,
        tcp_connect_probe,
        async_tcp_connect_probe,
        banner_probe,
        async_banner_probe,
        # Protocol probes
        DnsQueryConfigPy as DnsQueryConfig,
        DnsRecordPy as DnsRecord,
        DnsQueryResultPy as DnsQueryResult,
        TlsProbeConfigPy as TlsProbeConfig,
        CertificateInfoPy as CertificateInfo,
        CertificateChainEntryPy as CertificateChainEntry,
        TlsProbeResultPy as TlsProbeResult,
        TlsIssuePy as TlsIssue,
        HttpProbeConfigPy as HttpProbeConfig,
        HttpProbeResultPy as HttpProbeResult,
        UdpProbeConfigPy as UdpProbeConfig,
        UdpProbeResultPy as UdpProbeResult,
        dns_query,
        async_dns_query,
        tls_probe,
        async_tls_probe,
        http_probe,
        async_http_probe,
        udp_probe,
        async_udp_probe,
    )
except (AttributeError, ImportError):
    pass

# HTTP client (feature-gated: websocket or http-api)
try:
    from .._core import (
        HttpRequestPy as HttpRequest,
        HttpHeadersPy as HttpHeaders,
        HttpResponsePy as HttpResponse,
        HttpCookiePy as HttpCookie,
        RedirectEntryPy as RedirectEntry,
        TlsMetadataPy as TlsMetadata,
        HttpTimingPy as HttpTiming,
        HttpClientConfigPy as HttpClientConfig,
        HttpClientPy as HttpClient,
        AsyncHttpClientPy as AsyncHttpClient,
        RedactConfigPy as RedactConfig,
        create_http_client,
        async_create_http_client,
    )
except (AttributeError, ImportError):
    pass

# WebSocket sessions (feature-gated: websocket)
try:
    from .._core import (
        WebSocketSessionConfigPy as WebSocketSessionConfig,
        WebSocketMessagePy as WebSocketMessage,
        WebSocketFramePy as WebSocketFrame,
        WebSocketCloseInfoPy as WebSocketCloseInfo,
        WebSocketHandshakePy as WebSocketHandshake,
        WebSocketSessionPy as WebSocketSession,
        AsyncWebSocketSessionPy as AsyncWebSocketSession,
        WebSocketAssessmentConfigPy as WebSocketAssessmentConfig,
        WebSocketAssessmentResultPy as WebSocketAssessmentResult,
        websocket_assess,
        async_websocket_assess,
        websocket_probe,
        async_websocket_probe,
        websocket_fuzz,
        async_websocket_fuzz,
    )
except (AttributeError, ImportError):
    pass
