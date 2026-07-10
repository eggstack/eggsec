"""Type stubs for GraphQL security assessment module."""

from typing import List, Optional

class GraphQLVulnerability:
    """GraphQL vulnerability types."""
    Introspection: str
    QueryInjection: str
    DepthLimitBypass: str
    AliasBypass: str
    BatchBypass: str
    DirectiveInjection: str
    FieldSuggestion: str
    AliasOverload: str

class GraphQLTestResult:
    """Result from a GraphQL security test."""
    vulnerability: str
    success: bool
    query: str
    response_snippet: Optional[str]
    severity: str
    description: str

class GraphQLType:
    """A GraphQL type from introspection."""
    name: str
    kind: str
    description: Optional[str]
    fields: List[GraphQLField]

class GraphQLField:
    """A field in a GraphQL type."""
    name: str
    type_name: str
    is_nullable: bool
    is_list: bool
    description: Optional[str]
    args: List[GraphQLArg]

class GraphQLArg:
    """An argument to a GraphQL field."""
    name: str
    type_name: str
    default_value: Optional[str]

class GraphQLInputField:
    """An input field in a GraphQL input type."""
    name: str
    type_name: str
    is_required: bool
    default_value: Optional[str]

class GraphQLSchema:
    """Parsed GraphQL schema."""
    query_type: Optional[str]
    mutation_type: Optional[str]
    subscription_type: Optional[str]
    types: List[GraphQLType]

class GraphQLTestConfig:
    """Configuration for GraphQL security testing."""
    endpoint: str
    enable_introspection: bool
    enable_depth_bypass: bool
    enable_alias_overload: bool
    timeout_secs: int
    def __init__(
        self,
        endpoint: str,
        *,
        enable_introspection: bool = True,
        enable_depth_bypass: bool = True,
        enable_alias_overload: bool = True,
        timeout_secs: int = 30,
    ) -> None: ...

def graphql_test(config: GraphQLTestConfig) -> List[GraphQLTestResult]:
    """Run GraphQL security tests."""
    ...

async def async_graphql_test(config: GraphQLTestConfig) -> List[GraphQLTestResult]:
    """Run GraphQL security tests (async)."""
    ...
