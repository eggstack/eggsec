# Output Module Override

Specialized guidance for the report generation module.

## Report Formats

`output/` supports multiple output formats:
- JSON
- HTML
- SARIF
- JUnit

## Severity Re-export

`output/agent::Severity` and `output::trend::Severity` re-export from `crate::types::Severity`.