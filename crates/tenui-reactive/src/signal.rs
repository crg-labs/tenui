use std::{cell::RefCell, rc::Rc};

use crate::invalidation::{Invalidation, InvalidationSink};

type Listener = Box<dyn Fn()>;
type SignalSinkBinding = (u64, Invalidation, Rc<dyn InvalidationSink>);
type BatchedInvalidation = (Rc<dyn InvalidationSink>, u64, Invalidation);

/// A fine-grained reactive signal holding a typed value.
pub struct Signal<T> {
    value: RefCell<T>,
    listeners: RefCell<Vec<Rc<Listener>>>,
    sink_binding: RefCell<Option<SignalSinkBinding>>,
}

impl<T: Clone> Signal<T> {
    pub fn new(initial: T) -> Rc<Self> {
        Rc::new(Self {
            value: RefCell::new(initial),
            listeners: RefCell::new(Vec::new()),
            sink_binding: RefCell::new(None),
        })
    }

    /// Binds signal mutations to an invalidation sink.
    pub fn bind_invalidation(&self, node_id: u64, flags: Invalidation, sink: Rc<dyn InvalidationSink>) {
        *self.sink_binding.borrow_mut() = Some((node_id, flags, sink));
    }

    pub fn get(&self) -> T {
        self.value.borrow().clone()
    }

    pub fn set(&self, new_val: T)
    where
        T: PartialEq,
    {
        if *self.value.borrow() == new_val {
            return;
        }
        *self.value.borrow_mut() = new_val;
        self.notify();
    }

    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        f(&mut self.value.borrow_mut());
        self.notify();
    }

    pub fn subscribe<F>(&self, f: F)
    where
        F: Fn() + 'static,
    {
        self.listeners.borrow_mut().push(Rc::new(Box::new(f)));
    }

    fn notify(&self) {
        let binding = self.sink_binding.borrow().clone();
        let listeners: Vec<Rc<Listener>> = self.listeners.borrow().clone();

        let in_batch = IN_BATCH.with(|b| b.get());
        if in_batch {
            if let Some((node_id, flags, sink)) = &binding {
                let sink_clone = Rc::clone(sink);
                BATCH_DIRTY.with(|bd| {
                    bd.borrow_mut().push((sink_clone, *node_id, *flags));
                });
            }
            if !listeners.is_empty() {
                BATCH_NOTIFICATIONS.with(|bn| {
                    bn.borrow_mut().push(Box::new(move || {
                        for listener in listeners {
                            listener();
                        }
                    }));
                });
            }
        } else {
            if let Some((node_id, flags, sink)) = &binding {
                sink.mark_dirty(*node_id, *flags);
            }
            for listener in listeners {
                listener();
            }
        }
    }
}

thread_local! {
    static IN_BATCH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static BATCH_NOTIFICATIONS: RefCell<Vec<Box<dyn FnOnce()>>> = RefCell::new(Vec::new());
    static BATCH_DIRTY: RefCell<Vec<BatchedInvalidation>> = RefCell::new(Vec::new());
}

/// Transactional evaluation context providing atomic signal batching.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReactiveContext;

impl ReactiveContext {
    pub fn new() -> Self {
        Self
    }

    /// Queues signal notifications and dispatches exactly one consolidated invalidation pass.
    pub fn batch<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        batch(f)
    }
}

/// Dispatches an atomic signal batch closure, suppressing intermediate invalidations until completion.
pub fn batch<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    IN_BATCH.with(|in_b| {
        if in_b.get() {
            return f();
        }
        in_b.set(true);
        let res = f();
        in_b.set(false);

        let pending_dirty = BATCH_DIRTY.with(|bd| bd.borrow_mut().drain(..).collect::<Vec<_>>());
        for (sink, node_id, flags) in pending_dirty {
            sink.mark_dirty(node_id, flags);
        }

        let pending_notifs = BATCH_NOTIFICATIONS.with(|bn| bn.borrow_mut().drain(..).collect::<Vec<_>>());
        for notif in pending_notifs {
            notif();
        }

        res
    })
}

/// A derived/computed reactive value that updates when its upstream dependencies change.
pub struct Derived<T> {
    compute: Box<dyn Fn() -> T>,
}

impl<T> Derived<T> {
    pub fn new<F>(compute: F) -> Self
    where
        F: Fn() -> T + 'static,
    {
        Self {
            compute: Box::new(compute),
        }
    }

    pub fn get(&self) -> T {
        (self.compute)()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::*;
    use crate::invalidation::{Invalidation, InvalidationSink};

    struct TestSink {
        calls: RefCell<Vec<(u64, Invalidation)>>,
    }

    impl TestSink {
        fn new() -> Rc<Self> {
            Rc::new(Self {
                calls: RefCell::new(Vec::new()),
            })
        }

        fn take_calls(&self) -> Vec<(u64, Invalidation)> {
            self.calls.borrow_mut().drain(..).collect()
        }
    }

    impl InvalidationSink for TestSink {
        fn mark_dirty(&self, node_id: u64, flags: Invalidation) {
            self.calls.borrow_mut().push((node_id, flags));
        }
    }

    #[test]
    fn signal_initial_value() {
        let sig = Signal::new(42);
        assert_eq!(sig.get(), 42);
    }

    #[test]
    fn set_same_value_is_noop() {
        let sig = Signal::new(10);
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        sig.subscribe(move || {
            c.set(c.get() + 1);
        });
        sig.set(10);
        assert_eq!(count.get(), 0);
    }

    #[test]
    fn set_different_value_fires_listener() {
        let sig = Signal::new(1);
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        sig.subscribe(move || {
            c.set(c.get() + 1);
        });
        sig.set(2);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn update_always_fires_listener() {
        let sig = Signal::new(5);
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        sig.subscribe(move || {
            c.set(c.get() + 1);
        });
        sig.update(|v| *v += 0);
        assert_eq!(count.get(), 1, "update fires even without value change");
    }

    #[test]
    fn multiple_listeners() {
        let sig = Signal::new(0);
        let a = Rc::new(Cell::new(false));
        let b = Rc::new(Cell::new(false));
        let a2 = a.clone();
        let b2 = b.clone();
        sig.subscribe(move || a2.set(true));
        sig.subscribe(move || b2.set(true));
        sig.set(1);
        assert!(a.get());
        assert!(b.get());
    }

    #[test]
    fn derived_computes_from_signal() {
        let sig = Signal::new(3);
        let sig2 = sig.clone();
        let derived = Derived::new(move || sig2.get() * 2);
        assert_eq!(derived.get(), 6);
        sig.set(10);
        assert_eq!(derived.get(), 20);
    }

    #[test]
    fn batch_defers_notifications() {
        let sig = Signal::new(0);
        let observed = Rc::new(Cell::new(0));
        let obs = observed.clone();
        let sig2 = sig.clone();
        sig.subscribe(move || {
            obs.set(sig2.get());
        });

        batch(|| {
            sig.set(1);
            assert_eq!(observed.get(), 0, "listener deferred during batch");
            sig.set(5);
            assert_eq!(observed.get(), 0, "still deferred");
        });
        assert_eq!(observed.get(), 5, "listener fires after batch");
    }

    #[test]
    fn batch_defers_sink_marks() {
        let sig = Signal::new(0);
        let sink = TestSink::new();
        sig.bind_invalidation(42, Invalidation::PAINT, sink.clone());

        batch(|| {
            sig.set(1);
            assert!(sink.take_calls().is_empty(), "sink deferred during batch");
            sig.set(2);
        });

        let calls = sink.take_calls();
        assert_eq!(calls.len(), 2);
        assert!(calls.iter().all(|&(id, f)| id == 42 && f == Invalidation::PAINT));
    }

    #[test]
    fn nested_batch_defers_to_outer() {
        let sig = Signal::new(0);
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        sig.subscribe(move || {
            c.set(c.get() + 1);
        });

        batch(|| {
            sig.set(1);
            batch(|| {
                sig.set(2);
            });
            assert_eq!(count.get(), 0, "inner batch does not fire in outer scope");
        });
        assert!(count.get() > 0, "fires after outer batch completes");
    }

    #[test]
    fn reactive_context_batch_delegates() {
        let ctx = ReactiveContext::new();
        let sig = Signal::new(0);
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        sig.subscribe(move || {
            c.set(c.get() + 1);
        });

        ctx.batch(|| {
            sig.set(1);
            assert_eq!(count.get(), 0);
        });
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn bind_invalidation_routes_to_sink() {
        let sig = Signal::new("a".to_string());
        let sink = TestSink::new();
        sig.bind_invalidation(99, Invalidation::LAYOUT, sink.clone());
        sig.set("b".to_string());
        let calls = sink.take_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], (99, Invalidation::LAYOUT));
    }

    #[test]
    fn batch_returns_value() {
        let result = batch(|| 42);
        assert_eq!(result, 42);
    }
}
