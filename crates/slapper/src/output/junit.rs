//! JUnit XML report generation.
//!
//! ## XXE Safety
//!
//! This module uses [`quick_xml::Writer`] for XML generation. The Writer operates in
//! **write-only mode** - it does not parse, validate, or expand XML entities during
//! output generation. XML is serialized directly from in-memory data structures without
//! any entity expansion or external resource fetching, making it immune to XXE attacks
//! when generating reports.

use chrono::{DateTime, Utc};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JUnitReport {
    pub name: String,
    pub tests: u32,
    pub failures: u32,
    pub errors: u32,
    pub time: f64,
    pub test_suites: Vec<JUnitTestSuite>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JUnitTestSuite {
    pub name: String,
    pub tests: u32,
    pub failures: u32,
    pub errors: u32,
    pub skipped: u32,
    pub time: f64,
    pub timestamp: DateTime<Utc>,
    pub hostname: String,
    pub test_cases: Vec<JUnitTestCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JUnitTestCase {
    pub name: String,
    pub classname: String,
    pub time: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<JUnitFailure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JUnitError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<JUnitSkipped>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_out: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_err: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JUnitFailure {
    pub message: String,
    #[serde(rename = "type")]
    pub failure_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JUnitError {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JUnitSkipped {
    pub message: String,
}

pub struct JUnitBuilder {
    name: String,
    test_suites: FxHashMap<String, JUnitTestSuiteBuilder>,
}

struct JUnitTestSuiteBuilder {
    name: String,
    test_cases: Vec<JUnitTestCase>,
    start_time: DateTime<Utc>,
}

impl JUnitBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            test_suites: FxHashMap::default(),
        }
    }

    pub fn with_report(mut self, report: &crate::pipeline::PipelineReport) -> Self {
        let suite_name = format!("slapper-scan-{}", report.target);

        for port in &report.open_ports {
            if port.status == "open" {
                self = self.add_test_case(
                    &suite_name,
                    &format!("port_{}_open", port.port),
                    "port_scan",
                    0.0,
                    JUnitTestResult::Passed,
                );
            }
        }

        for service in &report.services {
            let service_name = format!(
                "{}_v{}",
                service.service,
                service.version.as_deref().unwrap_or("unknown")
            );
            self = self.add_test_case(
                &suite_name,
                &service_name,
                "fingerprint",
                0.0,
                JUnitTestResult::Passed,
            );
        }

        for endpoint in &report.endpoints {
            let test_name = format!(
                "{} {} - {}",
                endpoint.path, endpoint.status_code, endpoint.status_text
            );
            let result = if endpoint.status_code >= 400 {
                JUnitTestResult::Failed {
                    message: format!("Endpoint returned error status: {}", endpoint.status_code),
                    failure_type: "HttpError".to_string(),
                    text: Some(format!("Path: {}", endpoint.path)),
                }
            } else {
                JUnitTestResult::Passed
            };
            self = self.add_test_case(
                &suite_name,
                &test_name,
                "endpoints",
                endpoint.response_time_ms as f64 / 1000.0,
                result,
            );
        }

        self
    }

    pub fn add_test_case(
        mut self,
        suite_name: &str,
        test_name: &str,
        classname: &str,
        time_secs: f64,
        result: JUnitTestResult,
    ) -> Self {
        let suite = self
            .test_suites
            .entry(suite_name.to_string())
            .or_insert_with(|| JUnitTestSuiteBuilder {
                name: suite_name.to_string(),
                test_cases: Vec::new(),
                start_time: Utc::now(),
            });

        let test_case = JUnitTestCase {
            name: test_name.to_string(),
            classname: classname.to_string(),
            time: time_secs,
            failure: None,
            error: None,
            skipped: None,
            system_out: None,
            system_err: None,
        };

        let test_case = match result {
            JUnitTestResult::Passed => test_case,
            JUnitTestResult::Failed {
                message,
                failure_type,
                text,
            } => JUnitTestCase {
                failure: Some(JUnitFailure {
                    message,
                    failure_type,
                    text,
                }),
                ..test_case
            },
            JUnitTestResult::Error {
                message,
                error_type,
                text,
            } => JUnitTestCase {
                error: Some(JUnitError {
                    message,
                    error_type,
                    text,
                }),
                ..test_case
            },
            JUnitTestResult::Skipped { message } => JUnitTestCase {
                skipped: Some(JUnitSkipped { message }),
                ..test_case
            },
        };

        suite.test_cases.push(test_case);
        self
    }

    pub fn add_finding(
        self,
        suite_name: &str,
        finding_type: &str,
        severity: &str,
        target: &str,
        description: &str,
    ) -> Self {
        self.add_test_case(
            suite_name,
            &format!("{}_{}", finding_type, target),
            finding_type,
            0.0,
            JUnitTestResult::Failed {
                message: format!("[{}] {}", severity, description),
                failure_type: finding_type.to_string(),
                text: Some(format!("Target: {}\nDescription: {}", target, description)),
            },
        )
    }

    pub fn build(self) -> JUnitReport {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "unknown".to_string());

        let suites: Vec<JUnitTestSuite> = self
            .test_suites
            .into_values()
            .map(|builder| {
                let tests = builder.test_cases.len() as u32;
                let failures = builder
                    .test_cases
                    .iter()
                    .filter(|tc| tc.failure.is_some())
                    .count() as u32;
                let errors = builder
                    .test_cases
                    .iter()
                    .filter(|tc| tc.error.is_some())
                    .count() as u32;
                let skipped = builder
                    .test_cases
                    .iter()
                    .filter(|tc| tc.skipped.is_some())
                    .count() as u32;
                let time: f64 = builder.test_cases.iter().map(|tc| tc.time).sum();

                JUnitTestSuite {
                    name: builder.name,
                    tests,
                    failures,
                    errors,
                    skipped,
                    time,
                    timestamp: builder.start_time,
                    hostname: hostname.clone(),
                    test_cases: builder.test_cases,
                }
            })
            .collect();

        let total_tests = suites.iter().map(|s| s.tests).sum();
        let total_failures = suites.iter().map(|s| s.failures).sum();
        let total_errors = suites.iter().map(|s| s.errors).sum();
        let total_time = suites.iter().map(|s| s.time).sum();

        JUnitReport {
            name: self.name,
            tests: total_tests,
            failures: total_failures,
            errors: total_errors,
            time: total_time,
            test_suites: suites,
        }
    }
}

pub enum JUnitTestResult {
    Passed,
    Failed {
        message: String,
        failure_type: String,
        text: Option<String>,
    },
    Error {
        message: String,
        error_type: String,
        text: Option<String>,
    },
    Skipped {
        message: String,
    },
}

impl JUnitReport {
    pub fn to_xml(&self) -> Result<String, quick_xml::Error> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        let mut testsuites = BytesStart::new("testsuites");
        testsuites.push_attribute(("name", self.name.as_str()));
        testsuites.push_attribute(("tests", self.tests.to_string().as_str()));
        testsuites.push_attribute(("failures", self.failures.to_string().as_str()));
        testsuites.push_attribute(("errors", self.errors.to_string().as_str()));
        testsuites.push_attribute(("time", format!("{:.3}", self.time).as_str()));

        writer.write_event(Event::Start(testsuites))?;

        for suite in &self.test_suites {
            self.write_testsuite(&mut writer, suite)?;
        }

        writer.write_event(Event::End(BytesEnd::new("testsuites")))?;

        let result = writer.into_inner().into_inner();
        match String::from_utf8(result) {
            Ok(s) => Ok(s),
            Err(e) => Err(quick_xml::Error::Io(std::sync::Arc::new(
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            ))),
        }
    }

    fn write_testsuite<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        suite: &JUnitTestSuite,
    ) -> Result<(), quick_xml::Error> {
        let mut testsuite = BytesStart::new("testsuite");
        testsuite.push_attribute(("name", suite.name.as_str()));
        testsuite.push_attribute(("tests", suite.tests.to_string().as_str()));
        testsuite.push_attribute(("failures", suite.failures.to_string().as_str()));
        testsuite.push_attribute(("errors", suite.errors.to_string().as_str()));
        testsuite.push_attribute(("skipped", suite.skipped.to_string().as_str()));
        testsuite.push_attribute(("time", format!("{:.3}", suite.time).as_str()));
        testsuite.push_attribute(("timestamp", suite.timestamp.to_rfc3339().as_str()));
        testsuite.push_attribute(("hostname", suite.hostname.as_str()));

        writer.write_event(Event::Start(testsuite))?;

        for test_case in &suite.test_cases {
            self.write_testcase(writer, test_case)?;
        }

        writer.write_event(Event::End(BytesEnd::new("testsuite")))?;

        Ok(())
    }

    fn write_testcase<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        test_case: &JUnitTestCase,
    ) -> Result<(), quick_xml::Error> {
        let mut testcase = BytesStart::new("testcase");
        testcase.push_attribute(("name", test_case.name.as_str()));
        testcase.push_attribute(("classname", test_case.classname.as_str()));
        testcase.push_attribute(("time", format!("{:.3}", test_case.time).as_str()));

        writer.write_event(Event::Start(testcase))?;

        if let Some(ref failure) = test_case.failure {
            let mut failure_elem = BytesStart::new("failure");
            failure_elem.push_attribute(("message", failure.message.as_str()));
            failure_elem.push_attribute(("type", failure.failure_type.as_str()));
            writer.write_event(Event::Start(failure_elem))?;

            if let Some(ref text) = failure.text {
                writer.write_event(Event::Text(BytesText::new(text)))?;
            }

            writer.write_event(Event::End(BytesEnd::new("failure")))?;
        }

        if let Some(ref error) = test_case.error {
            let mut error_elem = BytesStart::new("error");
            error_elem.push_attribute(("message", error.message.as_str()));
            error_elem.push_attribute(("type", error.error_type.as_str()));
            writer.write_event(Event::Start(error_elem))?;

            if let Some(ref text) = error.text {
                writer.write_event(Event::Text(BytesText::new(text)))?;
            }

            writer.write_event(Event::End(BytesEnd::new("error")))?;
        }

        if let Some(ref skipped) = test_case.skipped {
            let mut skipped_elem = BytesStart::new("skipped");
            skipped_elem.push_attribute(("message", skipped.message.as_str()));
            writer.write_event(Event::Empty(skipped_elem))?;
        }

        writer.write_event(Event::End(BytesEnd::new("testcase")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_junit_builder() {
        let report = JUnitBuilder::new("Security Tests")
            .add_test_case(
                "SQL Injection",
                "test_sqli_param_id",
                "SQLInjection",
                0.5,
                JUnitTestResult::Failed {
                    message: "SQL injection vulnerability".to_string(),
                    failure_type: "SQLI".to_string(),
                    text: Some("Payload: ' OR 1=1--".to_string()),
                },
            )
            .add_test_case(
                "XSS",
                "test_xss_param_search",
                "XSS",
                0.3,
                JUnitTestResult::Passed,
            )
            .build();

        assert_eq!(report.name, "Security Tests");
        assert_eq!(report.tests, 2);
        assert_eq!(report.failures, 1);
    }

    #[test]
    fn test_junit_xml_output() {
        let report = JUnitBuilder::new("Test")
            .add_test_case("Suite", "test1", "Class", 1.0, JUnitTestResult::Passed)
            .build();

        let xml = report.to_xml().unwrap();
        assert!(xml.contains("<?xml"));
        assert!(xml.contains("<testsuites"));
    }
}
