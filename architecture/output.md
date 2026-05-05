# Output & Reporting Module

The Output module handles the formatting, deduplication, and export of security findings and scan data into various standardized formats.

## Supported Formats (`src/output/`)

Slapper supports a wide range of output formats to integrate with different tools and workflows:

- **JSON & SARIF (`sarif.rs`)**: For programmatic consumption and integration with IDEs or vulnerability management platforms.
- **HTML (`html.rs`)**: For human-readable, interactive reports with charts and tables.
- **Markdown (`markdown.rs`)**: For easy copy-pasting into documentation or tickets.
- **PDF (`pdf.rs`)**: For formal reporting requirements.
- **CSV (`csv.rs`)**: For spreadsheet-based analysis.
- **JUnit (`junit.rs`)**: For integration with CI/CD pipelines to fail builds on critical findings.

## Core Features

### Deduplication (`dedup.rs`)

Automatically identifies and groups duplicate findings (e.g., the same vulnerability found through different paths or payloads) to reduce noise in reports.

### Templates (`template.rs`)

Allows for customizing the content and style of HTML and Markdown reports using a templating engine.

### Attack Graphs (`attack_graph.rs`)

Visualizes the relationships between different findings to show potential attack paths (e.g., how a recon finding leads to a successful fuzzing attempt).

### Trend Analysis (`trend.rs`)

Compares current results with historical data to show how the security posture of a target changes over time.

## Integration

The Output module is typically the final stage in any Slapper operation. It can also be used independently to convert or merge existing Slapper result files.
