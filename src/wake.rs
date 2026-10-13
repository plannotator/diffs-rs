use std::sync::mpsc::SyncSender;

pub(crate) struct NotifyOnExit(pub SyncSender<()>);

impl Drop for NotifyOnExit {
    fn drop(&mut self) {
        let _ = self.0.try_send(());
    }
}
