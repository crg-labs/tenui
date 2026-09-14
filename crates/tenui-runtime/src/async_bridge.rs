use std::sync::mpsc;

pub enum AppMessage<M> {
    User(M),
    Redraw,
    Quit,
}

pub struct AsyncBridge<M> {
    tx: mpsc::Sender<AppMessage<M>>,
    rx: mpsc::Receiver<AppMessage<M>>,
}

impl<M> AsyncBridge<M> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx }
    }

    pub fn sender(&self) -> AppMessageSender<M> {
        AppMessageSender { tx: self.tx.clone() }
    }

    pub fn try_recv(&self) -> Option<AppMessage<M>> {
        self.rx.try_recv().ok()
    }

    pub fn drain(&self) -> Vec<AppMessage<M>> {
        let mut msgs = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            msgs.push(msg);
        }
        msgs
    }
}

impl<M> Default for AsyncBridge<M> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct AppMessageSender<M> {
    tx: mpsc::Sender<AppMessage<M>>,
}

impl<M> AppMessageSender<M> {
    pub fn send(&self, msg: M) -> Result<(), mpsc::SendError<AppMessage<M>>> {
        self.tx.send(AppMessage::User(msg))
    }

    pub fn request_redraw(&self) -> Result<(), mpsc::SendError<AppMessage<M>>> {
        self.tx.send(AppMessage::Redraw)
    }

    pub fn request_quit(&self) -> Result<(), mpsc::SendError<AppMessage<M>>> {
        self.tx.send(AppMessage::Quit)
    }
}

#[cfg(feature = "tokio")]
impl<M: Send + 'static> AppMessageSender<M> {
    pub fn spawn_task<F, Fut>(&self, f: F)
    where
        F: FnOnce(AppMessageSender<M>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let sender = self.clone();
        tokio::spawn(f(sender));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_send_receive() {
        let bridge: AsyncBridge<String> = AsyncBridge::new();
        let sender = bridge.sender();

        sender.send("hello".to_string()).unwrap();
        sender.request_redraw().unwrap();
        sender.request_quit().unwrap();

        let msgs = bridge.drain();
        assert_eq!(msgs.len(), 3);
        assert!(matches!(&msgs[0], AppMessage::User(s) if s == "hello"));
        assert!(matches!(&msgs[1], AppMessage::Redraw));
        assert!(matches!(&msgs[2], AppMessage::Quit));
    }

    #[test]
    fn bridge_try_recv_empty() {
        let bridge: AsyncBridge<()> = AsyncBridge::new();
        assert!(bridge.try_recv().is_none());
    }
}
