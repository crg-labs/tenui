//! Out-of-Band Remote DevTools & Time-Travel Debugger Server.

use std::collections::VecDeque;

/// Performance telemetry recording Phase 1 Reflow vs Phase 2 Paint timings and byte counts.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FrameTelemetry {
    pub frame_index: u64,
    pub reflow_duration_us: u64,
    pub paint_duration_us: u64,
    pub bytes_emitted: usize,
    pub layout_invoked: bool,
}

/// An append-only bounded ring buffer for deterministic time-travel debugging.
pub struct TimeTravelRingBuffer<T> {
    capacity: usize,
    buffer: VecDeque<T>,
    cursor: usize,
}

impl<T: Clone> TimeTravelRingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            buffer: VecDeque::with_capacity(capacity),
            cursor: 0,
        }
    }

    /// Appends a new epoch state or event to the ring buffer.
    pub fn record(&mut self, item: T) {
        if self.buffer.len() == self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(item);
        self.cursor = self.buffer.len();
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Steps backward in time, returning the previous historical state.
    pub fn step_backward(&mut self) -> Option<&T> {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.buffer.get(self.cursor)
        } else {
            None
        }
    }

    /// Steps forward in time, returning the next historical state.
    pub fn step_forward(&mut self) -> Option<&T> {
        if self.cursor + 1 < self.buffer.len() {
            self.cursor += 1;
            self.buffer.get(self.cursor)
        } else {
            None
        }
    }

    pub fn current_cursor(&self) -> usize {
        self.cursor
    }
}

/// Out-of-band DevTools Inspector protocol message handler (JSON-RPC 2.0).
pub struct DevToolsProtocol;

impl DevToolsProtocol {
    /// Formats a JSON-RPC 2.0 telemetry response.
    pub fn format_telemetry_response(id: u64, telemetry: &FrameTelemetry) -> String {
        format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"frame\":{},\"reflow_us\":{},\"paint_us\":{},\"bytes\":{},\"layout\":{}}}}}",
            id,
            telemetry.frame_index,
            telemetry.reflow_duration_us,
            telemetry.paint_duration_us,
            telemetry.bytes_emitted,
            telemetry.layout_invoked
        )
    }

    /// Formats a JSON-RPC 2.0 layout inspection response.
    pub fn format_layout_node(id: u64, node_id: u64, x: u16, y: u16, width: u16, height: u16) -> String {
        format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"node_id\":{},\"x\":{},\"y\":{},\"w\":{},\"h\":{}}}}}",
            id, node_id, x, y, width, height
        )
    }
}
