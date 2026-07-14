# Feature-gated: AI post-processing
from typing import Any

class AiProviderPy:
    pass

class PluginLanguagePy:
    pass

class ScriptTargetPy:
    pass

class AiAnalysisResultPy:
    @property
    def summary(self) -> str: ...

class AiPayloadSuggestionPy:
    @property
    def payload(self) -> str: ...

class AiWafBypassSuggestionPy:
    @property
    def technique(self) -> str: ...

class AiCacheStatsPy:
    @property
    def hits(self) -> int: ...
    @property
    def misses(self) -> int: ...

class ScriptMetadataPy:
    @property
    def name(self) -> str: ...

class GeneratedScriptPy:
    @property
    def source(self) -> str: ...

class AiCachePy:
    pass

def ai_analyze_finding(finding: Any, provider: AiProviderPy = ...) -> AiAnalysisResultPy: ...
async def async_ai_analyze_finding(finding: Any, provider: AiProviderPy = ...) -> AiAnalysisResultPy: ...
def ai_generate_payloads(context: Any, provider: AiProviderPy = ...) -> list[AiPayloadSuggestionPy]: ...
def ai_suggest_waf_bypass(waf_info: Any, provider: AiProviderPy = ...) -> list[AiWafBypassSuggestionPy]: ...
def ai_generate_script(metadata: ScriptMetadataPy, language: PluginLanguagePy = ..., target: ScriptTargetPy = ...) -> GeneratedScriptPy: ...
