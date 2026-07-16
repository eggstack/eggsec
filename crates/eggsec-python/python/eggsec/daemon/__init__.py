"""Daemon client types for eggsec.

This submodule contains the persistent session host client and parity contracts.

Maturity: provisional (transport parity pending)
"""

# Daemon parity types (always available)
try:
    from .._core import (
        DaemonProtocolVersion,
        IdempotencyKey,
        DaemonSubmissionResult,
        ReconnectOptions,
        ReplayCursor,
        ReplayResult,
        DaemonEventPy as DaemonEvent,
        CancellationRequest,
        CancellationResult,
        TaskArtifactDescriptor,
        EventReplayInfo,
        DaemonHealthDetail,
    )
except (AttributeError, ImportError):
    pass

# Daemon client functions (feature-gated: daemon-client)
try:
    from .._core import (
        daemon_connect,
        async_daemon_health,
        async_daemon_declare_client,
        async_daemon_create_session,
        async_daemon_list_sessions,
        async_daemon_get_snapshot,
        async_daemon_close_session,
        async_daemon_submit_task,
        async_daemon_cancel_task,
        async_daemon_cancel_active,
        async_daemon_approve_policy,
        async_daemon_list_persisted_sessions,
        async_daemon_get_persisted_snapshot,
        async_daemon_subscribe,
        DaemonClientPy as DaemonClient,
        DaemonResponsePy as DaemonResponse,
        DaemonCapabilitiesPy as DaemonCapabilities,
        TaskHandlePy as TaskHandle,
        TaskStatusPy as TaskStatus,
        DaemonEventPy as DaemonEvent,
        SessionSummaryPy as SessionSummary,
        TransportMetadataPy as TransportMetadata,
    )
except (AttributeError, ImportError):
    pass

# Keep export list truthful for feature-gated builds
__all__ = [name for name in dir() if not name.startswith("_")]
