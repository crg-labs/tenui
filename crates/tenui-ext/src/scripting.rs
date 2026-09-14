use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// Value types supported by the embedded scripting bridge.
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptValue {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<ScriptValue>),
}

impl From<bool> for ScriptValue {
    fn from(b: bool) -> Self {
        ScriptValue::Bool(b)
    }
}

impl From<i64> for ScriptValue {
    fn from(i: i64) -> Self {
        ScriptValue::Int(i)
    }
}

impl From<f64> for ScriptValue {
    fn from(f: f64) -> Self {
        ScriptValue::Float(f)
    }
}

impl From<&str> for ScriptValue {
    fn from(s: &str) -> Self {
        ScriptValue::Str(s.to_string())
    }
}

impl From<String> for ScriptValue {
    fn from(s: String) -> Self {
        ScriptValue::Str(s)
    }
}

type CommandFn = Box<dyn Fn(&[ScriptValue]) -> Result<ScriptValue, String> + Send + Sync>;
type HookFn = Box<dyn Fn(&str, &[ScriptValue]) + Send + Sync>;
type ScriptSignalListener = Arc<dyn Fn(&ScriptValue) + Send + Sync>;

/// Reactive signal bridge entry for embedded scripting environments.
#[derive(Clone)]
struct ReactiveBridgeSignal {
    value: Arc<RwLock<ScriptValue>>,
    listeners: Arc<RwLock<Vec<ScriptSignalListener>>>,
}

/// Embedded scripting bridge providing command registration, reactive signal bindings,
/// and lifecycle hooks for Lua and Rhai.
#[derive(Default)]
pub struct ScriptEngineBridge {
    commands: HashMap<String, CommandFn>,
    signals: HashMap<String, ReactiveBridgeSignal>,
    hooks: Vec<HookFn>,
}

impl ScriptEngineBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a callable script command.
    pub fn register_command<F>(&mut self, name: impl Into<String>, callback: F)
    where
        F: Fn(&[ScriptValue]) -> Result<ScriptValue, String> + Send + Sync + 'static,
    {
        self.commands.insert(name.into(), Box::new(callback));
    }

    /// Executes a registered script command by name.
    pub fn execute_command(&self, name: &str, args: &[ScriptValue]) -> Result<ScriptValue, String> {
        if let Some(cmd) = self.commands.get(name) {
            cmd(args)
        } else {
            Err(format!("Command '{}' not found", name))
        }
    }

    /// Creates or binds a reactive signal accessible to script environments.
    pub fn create_signal(&mut self, name: impl Into<String>, initial: ScriptValue) {
        let name = name.into();
        self.signals.insert(
            name,
            ReactiveBridgeSignal {
                value: Arc::new(RwLock::new(initial)),
                listeners: Arc::new(RwLock::new(Vec::new())),
            },
        );
    }

    /// Reads current value of a script reactive signal.
    pub fn get_signal(&self, name: &str) -> Option<ScriptValue> {
        self.signals
            .get(name)
            .and_then(|sig| sig.value.read().ok().map(|v| v.clone()))
    }

    /// Mutates a script reactive signal, notifying all registered listeners.
    pub fn update_signal<F>(&self, name: &str, updater: F) -> Result<(), String>
    where
        F: FnOnce(&ScriptValue) -> ScriptValue,
    {
        if let Some(sig) = self.signals.get(name) {
            let new_val = {
                let mut val = sig.value.write().map_err(|e| e.to_string())?;
                let updated = updater(&val);
                *val = updated.clone();
                updated
            };

            // Notify listeners
            if let Ok(listeners) = sig.listeners.read() {
                for listener in listeners.iter() {
                    listener(&new_val);
                }
            }
            Ok(())
        } else {
            Err(format!("Signal '{}' not found", name))
        }
    }

    /// Registers an observer callback on a signal.
    pub fn listen_signal<F>(&self, name: &str, listener: F) -> Result<(), String>
    where
        F: Fn(&ScriptValue) + Send + Sync + 'static,
    {
        if let Some(sig) = self.signals.get(name) {
            let mut list = sig.listeners.write().map_err(|e| e.to_string())?;
            list.push(Arc::new(listener));
            Ok(())
        } else {
            Err(format!("Signal '{}' not found", name))
        }
    }

    /// Registers an event hook.
    pub fn register_hook<F>(&mut self, hook: F)
    where
        F: Fn(&str, &[ScriptValue]) + Send + Sync + 'static,
    {
        self.hooks.push(Box::new(hook));
    }

    /// Emits an event hook to all script listeners.
    pub fn emit_hook(&self, event_name: &str, args: &[ScriptValue]) {
        for hook in &self.hooks {
            hook(event_name, args);
        }
    }
}
