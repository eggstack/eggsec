#!/usr/bin/env python3
"""F1: Generate a comprehensive machine-readable compatibility baseline.

Captures every public symbol from the installed eggsec package with full
introspection: signatures, defaults, enum values, error codes, schema hashes,
protocol/ABI versions, maturity, and deprecation state.

Requirements:
    - eggsec installed (maturin develop)
    - Python 3.9+

Usage:
    python scripts/generate_python_compatibility_baseline.py
"""

from __future__ import annotations

import ast
import hashlib
import importlib
import inspect
import json
import os
import pkgutil
import sys
import textwrap
from pathlib import Path
from typing import Any


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_STUBS_DIR = Path(__file__).resolve().parent.parent / "crates" / "eggsec-python" / "python" / "eggsec"
_OUTPUT_PATH = Path(__file__).resolve().parent.parent / "crates" / "eggsec-python" / "tests" / "compatibility_baseline.json"

_SENTINEL = object()

# Stable-core operation IDs (canonical)
_STABLE_OPERATION_IDS = [
    "scan_ports", "scan_endpoints", "fingerprint_services",
    "recon_dns", "inspect_tls", "detect_technology",
    "detect_waf", "validate_waf", "fuzz_http", "load_test_http",
    "scan_git_secrets", "generate_sbom", "run_consolidated_recon",
    "graphql_test", "oauth_test", "auth_test",
    "db_probe", "nse_run", "scan_docker_image", "scan_kubernetes",
    "analyze_apk", "analyze_ipa",
]

_EXCEPTION_HIERARCHY: dict[str, str] = {
    "EggsecError": "Exception",
    "ConfigError": "EggsecError",
    "ScopeError": "EggsecError",
    "EnforcementError": "EggsecError",
    "NetworkError": "EggsecError",
    "ScanError": "EggsecError",
    "TimeoutError": "EggsecError",
    "FeatureUnavailableError": "EggsecError",
    "SerializationError": "EggsecError",
    "InternalError": "EggsecError",
    "CancellationError": "EggsecError",
}


def _hash_schema(obj: Any) -> str:
    """Deterministic schema hash for a JSON-serializable object."""
    blob = json.dumps(obj, sort_keys=True, default=str).encode()
    return hashlib.sha256(blob).hexdigest()[:16]


def _safe_import(name: str, module: str = "eggsec") -> Any:
    """Import name from module, return _SENTINEL on failure."""
    try:
        mod = importlib.import_module(module)
        return getattr(mod, name, _SENTINEL)
    except Exception:
        return _SENTINEL


def _get_stub_signatures() -> dict[str, str]:
    """Parse .pyi stubs and return {symbol_name: raw_signature_text}."""
    result: dict[str, str] = {}
    stubs_dir = _STUBS_DIR
    if not stubs_dir.is_dir():
        return result

    for stub_file in stubs_dir.glob("*.pyi"):
        try:
            source = stub_file.read_text(encoding="utf-8")
            tree = ast.parse(source, filename=str(stub_file))
        except Exception:
            continue

        for node in ast.iter_child_nodes(tree):
            if isinstance(node, ast.FunctionDef):
                sig = _format_func_sig(node)
                if sig:
                    result[node.name] = sig
            elif isinstance(node, (ast.ClassDef,)):
                for item in ast.iter_child_nodes(node):
                    if isinstance(item, ast.FunctionDef):
                        fqn = f"{node.name}.{item.name}"
                        sig = _format_func_sig(item)
                        if sig:
                            result[fqn] = sig
    return result


def _format_func_sig(node: ast.FunctionDef) -> str | None:
    """Format a function signature from AST, with defaults."""
    args = node.args
    parts: list[str] = []

    # positional-only args
    posonlyargs = getattr(args, "posonlyargs", [])
    defaults_offset = len(args.args) + len(posonlyargs) - len(args.defaults)
    idx = 0
    for i, arg in enumerate(posonlyargs):
        parts.append(arg.arg)
        di = i - len(posonlyargs) + len(args.defaults)
        if di >= 0 and di < len(args.defaults):
            parts[-1] += f" = {_unparse_default(args.defaults[di])}"

    if posonlyargs and args.args:
        parts.append("/")

    for i, arg in enumerate(args.args):
        parts.append(arg.arg)
        di = i - len(posonlyargs) + len(args.defaults) - len(posonlyargs)
        actual_di = i + len(posonlyargs) - (len(args.args) + len(posonlyargs) - len(args.defaults))
        if actual_di >= 0 and actual_di < len(args.defaults):
            parts[-1] += f" = {_unparse_default(args.defaults[actual_di])}"

    if args.vararg:
        parts.append(f"*{args.vararg.arg}")

    if args.kwonlyargs:
        if not args.vararg:
            parts.append("*")
        for i, arg in enumerate(args.kwonlyargs):
            s = arg.arg
            if i < len(args.kw_defaults) and args.kw_defaults[i] is not None:
                s += f" = {_unparse_default(args.kw_defaults[i])}"
            parts.append(s)

    if args.kwarg:
        parts.append(f"**{args.kwarg.arg}")

    ret = ""
    if node.returns:
        try:
            ret = f" -> {ast.unparse(node.returns)}"
        except Exception:
            pass

    return f"({', '.join(parts)}){ret}"


def _unparse_default(node: ast.expr) -> str:
    """Best-effort AST node -> string for default values."""
    try:
        return ast.unparse(node)
    except Exception:
        return "..."
    return "..."


def _kind_of(name: str, obj: Any) -> str:
    """Determine the kind of a public symbol."""
    if inspect.isclass(obj):
        return "class"
    if inspect.isfunction(obj) or inspect.isbuiltin(obj):
        return "function"
    if inspect.ismodule(obj):
        return "module"
    if isinstance(obj, property):
        return "property"
    if callable(obj):
        return "callable"
    # Check for enum-like
    if hasattr(obj, "__members__"):
        return "enum"
    return "constant"


def _is_deprecated(name: str) -> tuple[bool, str | None]:
    """Check if a name is deprecated via __init__.py _DEPRECATED_ALIASES."""
    try:
        mod = importlib.import_module("eggsec")
        aliases = getattr(mod, "_DEPRECATED_ALIASES", {})
        if name in aliases:
            canonical, new_path = aliases[name]
            return True, new_path
    except Exception:
        pass
    return False, None


def _stability_of(name: str, obj: Any) -> str:
    """Classify stability based on known patterns."""
    # Check api_surface for explicit classification
    try:
        mod = importlib.import_module("eggsec")
        surface = mod.api_surface()
        if name in surface:
            return surface[name].get("stability", "stable")
    except Exception:
        pass
    return "stable"


def _enum_values(obj: Any) -> dict[str, str]:
    """Extract enum member names and values."""
    values: dict[str, str] = {}
    if hasattr(obj, "__members__"):
        for member_name, member_val in obj.__members__.items():
            values[member_name] = str(member_val.value if hasattr(member_val, "value") else member_val)
    elif hasattr(obj, "__dict__"):
        for k, v in obj.__dict__.items():
            if not k.startswith("_") and isinstance(v, (str, int, float, bool)):
                values[k] = str(v)
    return values


def _get_operation_aliases() -> dict[str, str]:
    """Map operation_id -> list of function name aliases."""
    aliases: dict[str, list[str]] = {}
    try:
        mod = importlib.import_module("eggsec")
        surface = mod.api_surface()
        for name in surface:
            # async_ prefix pattern
            if name.startswith("async_") and name[6:] in surface:
                base = name[6:]
                if base not in aliases:
                    aliases[base] = []
                aliases[base].append(name)
    except Exception:
        pass
    return {k: v for k, v in aliases.items()}


def _introspect_function(name: str, obj: Any, stub_sigs: dict[str, str]) -> dict[str, Any]:
    """Introspect a function symbol into a baseline entry."""
    entry: dict[str, Any] = {
        "kind": "function",
        "stability": _stability_of(name, obj),
        "deprecated": False,
        "deprecated_replacement": None,
    }

    dep, repl = _is_deprecated(name)
    entry["deprecated"] = dep
    entry["deprecated_replacement"] = repl

    # Try to get signature from stubs first (more reliable for PyO3 bindings)
    if name in stub_sigs:
        entry["stub_signature"] = stub_sigs[name]
    elif name.endswith("Py") and name[:-2] in stub_sigs:
        entry["stub_signature"] = stub_sigs[name[:-2]]

    # Runtime introspection
    try:
        sig = inspect.signature(obj)
        params = []
        for pname, param in sig.parameters.items():
            p: dict[str, Any] = {"name": pname}
            if param.default is inspect.Parameter.empty:
                p["required"] = True
                p["default"] = None
            else:
                p["required"] = False
                p["default"] = repr(param.default) if not isinstance(param.default, str) else param.default

            kind_map = {
                inspect.Parameter.POSITIONAL_ONLY: "posonly",
                inspect.Parameter.POSITIONAL_OR_KEYWORD: "positional",
                inspect.Parameter.KEYWORD_ONLY: "keyword",
                inspect.Parameter.VAR_POSITIONAL: "variadic",
                inspect.Parameter.VAR_KEYWORD: "variadic_kw",
            }
            p["kind"] = kind_map.get(param.kind, "positional")

            if param.annotation is not inspect.Parameter.empty:
                try:
                    p["annotation"] = ast.unparse(param.annotation) if hasattr(param.annotation, '__class__') else str(param.annotation)
                except Exception:
                    p["annotation"] = str(param.annotation)

            params.append(p)
        entry["parameters"] = params

        if sig.return_annotation is not inspect.Parameter.empty:
            try:
                entry["return_annotation"] = ast.unparse(sig.return_annotation) if hasattr(sig.return_annotation, '__class__') else str(sig.return_annotation)
            except Exception:
                entry["return_annotation"] = str(sig.return_annotation)
    except (ValueError, TypeError):
        entry["parameters"] = "introspection_unavailable"

    return entry


def _introspect_class(name: str, obj: Any, stub_sigs: dict[str, str]) -> dict[str, Any]:
    """Introspect a class symbol."""
    entry: dict[str, Any] = {
        "kind": "class",
        "stability": _stability_of(name, obj),
        "deprecated": False,
        "deprecated_replacement": None,
        "methods": [],
        "properties": [],
        "enum_values": {},
        "protocol_support": [],
    }

    dep, repl = _is_deprecated(name)
    entry["deprecated"] = dep
    entry["deprecated_replacement"] = repl

    # Enum values
    vals = _enum_values(obj)
    if vals:
        entry["kind"] = "enum"
        entry["enum_values"] = vals

    # Methods and properties
    for attr_name in dir(obj):
        if attr_name.startswith("_") and attr_name not in ("__init__",):
            continue
        try:
            attr = getattr(obj, attr_name, None)
        except Exception:
            continue
        if attr is None:
            continue
        if isinstance(attr, property):
            entry["properties"].append(attr_name)
        elif callable(attr):
            method_sig_key = f"{name}.{attr_name}"
            if method_sig_key in stub_sigs:
                entry["methods"].append({"name": attr_name, "stub_signature": stub_sigs[method_sig_key]})
            else:
                entry["methods"].append({"name": attr_name})

    # Protocol support checks
    protocol_checks = {
        "__enter__": "context_manager",
        "__exit__": "context_manager",
        "__aenter__": "async_context_manager",
        "__aexit__": "async_context_manager",
        "__iter__": "iterable",
        "__next__": "iterator",
        "__len__": "sized",
        "__bool__": "bool_protocol",
        "__hash__": "hashable",
        "__eq__": "equality",
        "__repr__": "repr",
        "__str__": "str",
        "__dict__": "dict_serialization",
        "to_dict": "dict_serialization",
        "to_json": "json_serialization",
        "from_dict": "deserialization",
        "from_json": "deserialization",
    }
    for proto, category in protocol_checks.items():
        if hasattr(obj, proto):
            entry["protocol_support"].append(category)

    # Constructors
    try:
        init = getattr(obj, "__init__", None)
        if init:
            sig = inspect.signature(init)
            params = []
            for pname, param in sig.parameters.items():
                if pname == "self":
                    continue
                p: dict[str, Any] = {"name": pname}
                if param.default is inspect.Parameter.empty:
                    p["required"] = True
                    p["default"] = None
                else:
                    p["required"] = False
                    p["default"] = repr(param.default) if not isinstance(param.default, str) else param.default
                p["kind"] = "positional"
                params.append(p)
            entry["constructor_parameters"] = params
    except (ValueError, TypeError):
        entry["constructor_parameters"] = "introspection_unavailable"

    return entry


def _collect_operation_metadata() -> dict[str, Any]:
    """Collect operation metadata from OperationRegistry."""
    ops: dict[str, Any] = {}
    try:
        from eggsec import OperationRegistry
        all_ops = OperationRegistry.all_operations()
        for op in all_ops:
            op_id = getattr(op, "operation_id", None)
            if not op_id:
                continue
            ops[op_id] = {
                "label": getattr(op, "label", ""),
                "risk": str(getattr(op, "risk", "")),
                "required_feature": getattr(op, "required_feature", None) or "",
                "description": getattr(op, "description", ""),
            }
    except Exception:
        pass
    return ops


def _collect_domain_metadata() -> dict[str, Any]:
    """Collect domain maturity from domain_maturity()."""
    try:
        from eggsec import domain_maturity
        return domain_maturity()
    except Exception:
        return {}


def _collect_feature_matrix() -> dict[str, Any]:
    """Collect feature matrix."""
    try:
        from eggsec import feature_matrix
        return feature_matrix()
    except Exception:
        return {}


def _collect_version_constants() -> dict[str, Any]:
    """Collect version and schema constants."""
    result: dict[str, Any] = {}
    try:
        import eggsec
        result["schema_version"] = getattr(eggsec, "SCHEMA_VERSION", getattr(eggsec, "__schema_version__", ""))
        result["protocol_version"] = getattr(eggsec, "PROTOCOL_VERSION", getattr(eggsec, "__protocol_version__", ""))
        result["abi_version"] = getattr(eggsec, "ABI_VERSION", getattr(eggsec, "__abi_version__", ""))
        result["finding_schema_version"] = getattr(eggsec, "FINDING_SCHEMA_VERSION", "")
        result["event_schema_version"] = getattr(eggsec, "EVENT_SCHEMA_VERSION", "")
        result["version"] = getattr(eggsec, "__version__", "")
        version_info = getattr(eggsec, "__version_info__", None)
        if version_info:
            result["version_info"] = list(version_info)
    except Exception:
        pass
    return result


def _collect_exception_codes() -> dict[str, dict[str, Any]]:
    """Collect exception hierarchy with stable error codes."""
    result: dict[str, dict[str, Any]] = {}
    try:
        import eggsec
        for exc_name, parent_name in _EXCEPTION_HIERARCHY.items():
            exc_cls = getattr(eggsec, exc_name, None)
            if exc_cls is None:
                try:
                    from eggsec import errors
                    exc_cls = getattr(errors, exc_name, None)
                except Exception:
                    pass
            if exc_cls is not None:
                result[exc_name] = {
                    "parent": parent_name,
                    "stable": True,
                }
    except Exception:
        pass
    return result


def _collect_operation_aliases() -> dict[str, list[str]]:
    """Collect async/sync alias relationships."""
    aliases: dict[str, list[str]] = {}
    try:
        import eggsec
        surface = eggsec.api_surface()
        for name in surface:
            if name.startswith("async_"):
                base = name[6:]
                if base in surface:
                    aliases.setdefault(base, []).append(name)
    except Exception:
        pass
    return aliases


def _collect_schema_hashes() -> dict[str, str]:
    """Compute schema hashes for key DTOs."""
    hashes: dict[str, str] = {}
    try:
        import eggsec
        # Build a synthetic schema from stubs
        stubs_dir = _STUBS_DIR
        for stub_file in sorted(stubs_dir.glob("*.pyi")):
            try:
                source = stub_file.read_text(encoding="utf-8")
                # Hash the stub content as a proxy for schema identity
                h = hashlib.sha256(source.encode()).hexdigest()[:12]
                module_name = stub_file.stem
                hashes[f"stub:{module_name}"] = h
            except Exception:
                pass
    except Exception:
        pass
    return hashes


def _collect_wheel_profile_inventory() -> dict[str, Any]:
    """Collect wheel-profile feature inventories."""
    try:
        from eggsec import wheel_profile, features
        return {
            "profile": wheel_profile(),
            "compiled_features": sorted(features()),
        }
    except Exception:
        return {}


def generate_baseline() -> dict[str, Any]:
    """Generate the full compatibility baseline."""
    print("Collecting stub signatures...")
    stub_sigs = _get_stub_signatures()

    print("Introspecting eggsec public API...")
    surface = {}
    try:
        import eggsec
        surface = eggsec.api_surface()
    except Exception as e:
        print(f"Warning: api_surface() failed: {e}", file=sys.stderr)

    symbols: dict[str, Any] = {}
    all_names = sorted(surface.keys()) if surface else []

    # If api_surface returned names, introspect each
    for name in all_names:
        obj = _safe_import(name)
        if obj is _SENTINEL:
            continue

        kind = _kind_of(name, obj)
        if kind == "module":
            continue

        if kind in ("class", "enum"):
            entry = _introspect_class(name, obj, stub_sigs)
        elif kind == "function":
            entry = _introspect_function(name, obj, stub_sigs)
        elif kind == "constant":
            entry = {
                "kind": "constant",
                "stability": _stability_of(name, obj),
                "deprecated": False,
                "deprecated_replacement": None,
            }
            dep, repl = _is_deprecated(name)
            entry["deprecated"] = dep
            entry["deprecated_replacement"] = repl
            try:
                entry["value"] = repr(obj)
            except Exception:
                entry["value"] = "<unrepresentable>"
        else:
            entry = {
                "kind": kind,
                "stability": _stability_of(name, obj),
                "deprecated": False,
                "deprecated_replacement": None,
            }

        entry["name"] = name
        symbols[name] = entry

    # Also scan eggsec submodules for symbols not in api_surface
    submodule_names = [
        "eggsec.net", "eggsec.sessions", "eggsec.storage",
        "eggsec.reporting", "eggsec.daemon", "eggsec.experimental",
    ]
    for submod_name in submodule_names:
        try:
            submod = importlib.import_module(submod_name)
            for attr_name in dir(submod):
                if attr_name.startswith("_"):
                    continue
                if attr_name in symbols:
                    continue
                full_name = f"{submod_name}.{attr_name}"
                obj = getattr(submod, attr_name, None)
                if obj is None:
                    continue
                kind = _kind_of(full_name, obj)
                if kind == "module":
                    continue
                entry: dict[str, Any] = {
                    "name": full_name,
                    "kind": kind,
                    "stability": "provisional",
                    "deprecated": False,
                    "deprecated_replacement": None,
                }
                if kind in ("class", "enum"):
                    entry.update(_introspect_class(full_name, obj, stub_sigs))
                    entry["name"] = full_name
                elif kind == "function":
                    entry.update(_introspect_function(full_name, obj, stub_sigs))
                    entry["name"] = full_name
                symbols[full_name] = entry
        except ImportError:
            pass

    print(f"Collected {len(symbols)} symbols.")

    # Assemble baseline
    print("Collecting metadata...")
    baseline: dict[str, Any] = {
        "schema_version": "2.0",
        "generator": "generate_python_compatibility_baseline.py",
        "version_constants": _collect_version_constants(),
        "exception_hierarchy": _collect_exception_codes(),
        "operation_registry": _collect_operation_metadata(),
        "operation_aliases": _collect_operation_aliases(),
        "domain_maturity": _collect_domain_metadata(),
        "feature_matrix": _collect_feature_matrix(),
        "schema_hashes": _collect_schema_hashes(),
        "wheel_profile_inventory": _collect_wheel_profile_inventory(),
        "symbols": symbols,
    }

    return baseline


def main() -> int:
    baseline = generate_baseline()

    # Ensure output directory exists
    _OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)

    with open(_OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(baseline, f, indent=2, sort_keys=True, default=str)

    symbol_count = len(baseline.get("symbols", {}))
    print(f"Baseline written to {_OUTPUT_PATH}")
    print(f"  {symbol_count} symbols, {len(baseline.get('schema_hashes', {}))} schema hashes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
