#!/usr/bin/env python3
"""
Git Secret Scan through the Engine.

Demonstrates scanning a local git repository for leaked secrets using
the `scan_git_secrets` operation via Engine dispatch.

Requirements:
    - eggsec installed with `git-secrets` feature
    - A local git repository to scan

Usage:
    python git_secret_scan.py [/path/to/repo]
"""

import sys

import eggsec
from eggsec import Engine, Scope, GitSecretsScanRequest


def main():
    # Determine repository path
    repo_path = sys.argv[1] if len(sys.argv) > 1 else "."

    # Verify feature availability
    features = eggsec.features()
    if not features.get("git-secrets", False):
        print("Error: 'git-secrets' feature not compiled.")
        print("Build with: maturin develop --features git-secrets")
        sys.exit(1)

    # Create engine with scope allowing the target path
    scope = Scope.allow_hosts(["127.0.0.1"])
    engine = Engine(scope)

    # Build the request
    request = GitSecretsScanRequest(
        repo_path=repo_path,
        max_commits=500,  # Limit to last 500 commits
    )

    print(f"Scanning {repo_path} for secrets...")

    # Dispatch through engine
    result = engine.run_git_secrets_scan(request)

    # Check result status
    if result.status.name() == "Completed":
        report = result.payload
        print(f"\nScan complete: {report.total_secrets} secrets found")
        print(f"Commits scanned: {report.commits_scanned}")

        for secret in report.findings:
            print(f"\n  [{secret.confidence}] {secret.secret_type}")
            print(f"    File: {secret.file_path}:{secret.line_number}")
            print(f"    Commit: {secret.commit_hash[:12]}")
            if secret.description:
                print(f"    Description: {secret.description}")
    elif result.status.name() == "Failed":
        error = result.error
        print(f"Scan failed ({error.kind}): {error.message}")
        sys.exit(1)
    else:
        print(f"Unexpected status: {result.status.name()}")
        sys.exit(1)


if __name__ == "__main__":
    main()
