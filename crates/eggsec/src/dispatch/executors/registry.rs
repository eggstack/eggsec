use std::collections::HashMap;

use crate::dispatch::executor::OperationExecutor;

/// Registry that maps operation IDs to their executor adapters.
///
/// Built once at startup, then queried on every dispatch. The registry
/// ensures each operation ID is handled by exactly one executor.
pub struct ExecutorRegistry {
    executors: Vec<Box<dyn OperationExecutor>>,
    /// Maps canonical operation ID → index into `executors`.
    operation_to_executor: HashMap<String, usize>,
}

impl ExecutorRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            executors: Vec::new(),
            operation_to_executor: HashMap::new(),
        }
    }

    /// Register an executor, indexing all its operation IDs.
    ///
    /// # Panics
    ///
    /// Panics if any operation ID is already registered (duplicate detection).
    pub fn register(&mut self, executor: Box<dyn OperationExecutor>) {
        let idx = self.executors.len();
        for &op_id in executor.operation_ids() {
            if let Some(&existing) = self.operation_to_executor.get(op_id) {
                panic!(
                    "Duplicate operation ID '{}' registered in executor index {} and {}",
                    op_id, existing, idx
                );
            }
            self.operation_to_executor.insert(op_id.to_string(), idx);
        }
        self.executors.push(executor);
    }

    /// Find the executor that handles the given operation ID.
    pub fn find_executor(&self, operation_id: &str) -> Option<&dyn OperationExecutor> {
        self.operation_to_executor
            .get(operation_id)
            .map(|&idx| self.executors[idx].as_ref())
    }

    /// Return all registered operation IDs across all executors.
    pub fn all_operation_ids(&self) -> Vec<&str> {
        self.operation_to_executor
            .keys()
            .map(|s| s.as_str())
            .collect()
    }

    /// Return the number of registered executors.
    pub fn len(&self) -> usize {
        self.executors.len()
    }

    /// Return `true` if no executors are registered.
    pub fn is_empty(&self) -> bool {
        self.executors.is_empty()
    }
}

impl Default for ExecutorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OperationMetadata;

    struct TestExecutor {
        ids: Vec<&'static str>,
        meta: Vec<&'static OperationMetadata>,
    }

    impl OperationExecutor for TestExecutor {
        fn operation_ids(&self) -> &[&str] {
            &self.ids
        }
        fn metadata(&self) -> &[&OperationMetadata] {
            &self.meta
        }
    }

    #[test]
    fn registry_register_and_lookup() {
        let mut reg = ExecutorRegistry::new();
        let exec = Box::new(TestExecutor {
            ids: vec!["scan-ports", "fingerprint"],
            meta: vec![],
        });
        reg.register(exec);

        assert!(reg.find_executor("scan-ports").is_some());
        assert!(reg.find_executor("fingerprint").is_some());
        assert!(reg.find_executor("recon").is_none());
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
    }

    #[test]
    fn registry_all_operation_ids() {
        let mut reg = ExecutorRegistry::new();
        reg.register(Box::new(TestExecutor {
            ids: vec!["scan-ports", "fingerprint"],
            meta: vec![],
        }));
        reg.register(Box::new(TestExecutor {
            ids: vec!["recon"],
            meta: vec![],
        }));

        let mut ids = reg.all_operation_ids();
        ids.sort();
        assert_eq!(ids, vec!["fingerprint", "recon", "scan-ports"]);
    }

    #[test]
    #[should_panic(expected = "Duplicate operation ID")]
    fn registry_rejects_duplicate() {
        let mut reg = ExecutorRegistry::new();
        reg.register(Box::new(TestExecutor {
            ids: vec!["scan-ports"],
            meta: vec![],
        }));
        reg.register(Box::new(TestExecutor {
            ids: vec!["scan-ports"],
            meta: vec![],
        }));
    }
}
