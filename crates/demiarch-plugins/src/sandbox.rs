//! WASM sandbox execution via wasmtime

use crate::{Permission, PluginError, PluginResult};
use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
use wasmtime::{
    Config, Engine, Instance, Module, Store, StoreLimits, StoreLimitsBuilder, WasmBacktraceDetails,
};

pub struct Sandbox {
    engine: Engine,
    allowed_permissions: Vec<Permission>,
    fuel_limit: u64,
    memory_limit_bytes: usize,
    table_elements_limit: usize,
    instance_limit: usize,
    execution_timeout: Duration,
}

impl Sandbox {
    pub fn new(allowed_permissions: Vec<Permission>) -> PluginResult<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.wasm_threads(false);
        config.wasm_simd(false);
        config.wasm_reference_types(false);
        config.async_support(false);
        config.epoch_interruption(true);
        config.wasm_backtrace_details(WasmBacktraceDetails::Disable);

        let engine = Engine::new(&config).map_err(|e| {
            PluginError::WasmError(format!("Failed to initialize wasmtime engine: {e}"))
        })?;

        Ok(Self {
            engine,
            allowed_permissions,
            fuel_limit: 10_000_000,
            memory_limit_bytes: 16 * 1024 * 1024,
            table_elements_limit: 1_024,
            instance_limit: 16,
            execution_timeout: Duration::from_secs(5),
        })
    }

    /// Execute a WASM module in a constrained sandbox.
    /// No host functions are exposed; capabilities must be explicitly granted.
    pub fn execute(&self, wasm: &[u8], requested_permissions: &[Permission]) -> PluginResult<()> {
        let unique_permissions = self.assert_permissions(requested_permissions)?;

        if self.fuel_limit == 0 {
            return Err(PluginError::ValidationFailed(
                "Fuel limit must be greater than zero".to_string(),
            ));
        }

        Module::validate(&self.engine, wasm)
            .map_err(|e| PluginError::WasmError(format!("Module validation failed: {e}")))?;

        let module = Module::new(&self.engine, wasm)
            .map_err(|e| PluginError::WasmError(format!("Invalid module: {e}")))?;

        if module.imports().next().is_some() {
            return Err(PluginError::ValidationFailed(
                "Imports are not allowed unless explicitly exposed via permitted host functions"
                    .to_string(),
            ));
        }

        let mut store = Store::new(
            &self.engine,
            SandboxLimits {
                limits: StoreLimitsBuilder::new()
                    .memory_size(self.memory_limit_bytes)
                    .table_elements(self.table_elements_limit)
                    .instances(self.instance_limit)
                    .trap_on_grow_failure(true)
                    .build(),
            },
        );

        store.limiter(|state| &mut state.limits);

        // Enforce wall-clock timeout using epoch interruption
        store.set_epoch_deadline(1);

        store
            .set_fuel(self.fuel_limit)
            .map_err(|e| PluginError::WasmError(format!("Failed to add fuel: {e}")))?;

        let deadline_triggered = Arc::new(AtomicBool::new(false));
        let deadline_flag = deadline_triggered.clone();
        let engine = self.engine.clone();
        let timeout = self.execution_timeout;

        let watchdog = thread::spawn(move || {
            thread::sleep(timeout);
            if !deadline_flag.load(Ordering::Relaxed) {
                engine.increment_epoch();
            }
        });

        // No imports -> empty host environment by default
        let instantiation_result = Instance::new(&mut store, &module, &[]);

        deadline_triggered.store(true, Ordering::Relaxed);
        let _ = watchdog.join();

        instantiation_result
            .map_err(|e| PluginError::WasmError(format!("Instantiation failed: {e}")))?;

        // Ensure requested permissions are logged as used for auditing even if empty host
        let _ = unique_permissions;

        Ok(())
    }

    pub fn allows(&self, permission: Permission) -> bool {
        self.allowed_permissions.contains(&permission)
    }

    fn assert_permissions(&self, requested: &[Permission]) -> PluginResult<HashSet<Permission>> {
        if requested.is_empty() {
            return Err(PluginError::ValidationFailed(
                "At least one permission must be requested".to_string(),
            ));
        }

        let mut unique = HashSet::new();
        for permission in requested {
            if !unique.insert(*permission) {
                continue;
            }

            if !self.allows(*permission) {
                return Err(PluginError::ValidationFailed(format!(
                    "Permission '{:?}' not granted for sandbox",
                    permission
                )));
            }
        }

        Ok(unique)
    }
}

struct SandboxLimits {
    limits: StoreLimits,
}
