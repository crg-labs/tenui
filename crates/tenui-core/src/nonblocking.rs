use std::{
    io::{self, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{SyncSender, TrySendError, sync_channel},
    },
    thread::{self, JoinHandle},
};

/// Status of a frame write operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameStatus {
    Queued,
    DroppedBackpressure,
}

/// A non-blocking output sink that streams frames to a background writer thread.
///
/// If the output sink (e.g. high-latency SSH stdout or slow pipe) backpressures,
/// subsequent frames are automatically dropped without stalling the application's
/// event loop.
pub struct NonBlockingWriter {
    sender: Option<SyncSender<Vec<u8>>>,
    dropped_count: Arc<AtomicU64>,
    queued_count: Arc<AtomicU64>,
    running: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,
}

impl NonBlockingWriter {
    /// Creates a new non-blocking writer targeting `stdout` with the given queue capacity.
    /// A capacity of 1-2 frames ensures minimal latency and immediate frame-dropping on backpressure.
    pub fn stdout(capacity: usize) -> Self {
        Self::custom(io::stdout(), capacity)
    }

    /// Creates a new non-blocking writer for any `Write + Send + 'static` target.
    pub fn custom<W: Write + Send + 'static>(mut target: W, capacity: usize) -> Self {
        let (sender, receiver) = sync_channel::<Vec<u8>>(capacity);
        let dropped_count = Arc::new(AtomicU64::new(0));
        let queued_count = Arc::new(AtomicU64::new(0));
        let running = Arc::new(AtomicBool::new(true));

        let is_running = Arc::clone(&running);
        let worker_handle = thread::Builder::new()
            .name("tenui-writer".into())
            .spawn(move || {
                while is_running.load(Ordering::Relaxed) {
                    match receiver.recv() {
                        Ok(data) => {
                            let _ = target.write_all(&data);
                            let _ = target.flush();
                        }
                        Err(_) => break, // Channel disconnected
                    }
                }
                // Drain any remaining buffered frames before exiting
                while let Ok(data) = receiver.try_recv() {
                    let _ = target.write_all(&data);
                }
                let _ = target.flush();
            })
            .expect("spawn tenui-writer thread");

        Self {
            sender: Some(sender),
            dropped_count,
            queued_count,
            running,
            worker_handle: Some(worker_handle),
        }
    }

    /// Attempts to enqueue a frame to be written.
    ///
    /// Returns `FrameStatus::Queued` if accepted, or `FrameStatus::DroppedBackpressure`
    /// if the background writer is blocked and the queue is full.
    pub fn try_send_frame(&self, frame: Vec<u8>) -> FrameStatus {
        let sender = match self.sender.as_ref() {
            Some(s) => s,
            None => return FrameStatus::DroppedBackpressure,
        };
        match sender.try_send(frame) {
            Ok(()) => {
                self.queued_count.fetch_add(1, Ordering::Relaxed);
                FrameStatus::Queued
            }
            Err(TrySendError::Full(_)) => {
                self.dropped_count.fetch_add(1, Ordering::Relaxed);
                FrameStatus::DroppedBackpressure
            }
            Err(TrySendError::Disconnected(_)) => {
                self.dropped_count.fetch_add(1, Ordering::Relaxed);
                FrameStatus::DroppedBackpressure
            }
        }
    }

    pub fn dropped_frames(&self) -> u64 {
        self.dropped_count.load(Ordering::Relaxed)
    }

    pub fn queued_frames(&self) -> u64 {
        self.queued_count.load(Ordering::Relaxed)
    }
}

impl Write for NonBlockingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.try_send_frame(buf.to_vec()) {
            FrameStatus::Queued => Ok(buf.len()),
            FrameStatus::DroppedBackpressure => {
                // Return success so callers don't panic, but note the drop
                Ok(buf.len())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for NonBlockingWriter {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        // Drop the sender FIRST so the worker's blocking `recv()` returns `Err`
        // (channel disconnected) and the thread can exit. Joining before this
        // would deadlock: the live sender keeps `recv()` blocked forever.
        self.sender.take();
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, time::Duration};

    use super::*;

    /// Regression: dropping the writer must not deadlock while the worker thread
    /// is blocked in `recv()`. Run the construct+drop in a side thread and require
    /// it to complete within a generous timeout.
    #[test]
    fn test_drop_does_not_hang() {
        let (done_tx, done_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let writer = NonBlockingWriter::custom(io::sink(), 2);
            drop(writer);
            let _ = done_tx.send(());
        });
        assert!(
            done_rx.recv_timeout(Duration::from_secs(5)).is_ok(),
            "NonBlockingWriter::drop hung — worker thread was not released"
        );
    }

    /// Frames sent before drop are still flushed to the sink by the drain loop.
    #[test]
    fn test_frames_are_written() {
        use std::sync::{Arc, Mutex};

        #[derive(Clone)]
        struct SharedSink(Arc<Mutex<Vec<u8>>>);
        impl Write for SharedSink {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let store = Arc::new(Mutex::new(Vec::new()));
        let sink = SharedSink(store.clone());
        let writer = NonBlockingWriter::custom(sink, 8);
        assert_eq!(writer.try_send_frame(b"hello".to_vec()), FrameStatus::Queued);
        drop(writer); // drop drains + flushes remaining frames before joining
        assert_eq!(&*store.lock().unwrap(), b"hello");
    }
}
