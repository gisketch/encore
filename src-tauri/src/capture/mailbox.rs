use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError, TrySendError};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

pub struct LatestSender<T> {
    sender: Sender<T>,
    eviction_receiver: Receiver<T>,
    dropped: Arc<AtomicU64>,
}

impl<T> Clone for LatestSender<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            eviction_receiver: self.eviction_receiver.clone(),
            dropped: self.dropped.clone(),
        }
    }
}

impl<T> LatestSender<T> {
    pub fn send_latest(&self, mut item: T) {
        loop {
            match self.sender.try_send(item) {
                Ok(()) => return,
                Err(TrySendError::Full(returned)) => {
                    item = returned;
                    match self.eviction_receiver.try_recv() {
                        Ok(_) => {
                            self.dropped.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(TryRecvError::Empty) => continue,
                        Err(TryRecvError::Disconnected) => return,
                    }
                }
                Err(TrySendError::Disconnected(_)) => return,
            }
        }
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

pub fn latest_channel<T>(capacity: usize) -> (LatestSender<T>, Receiver<T>) {
    assert!(capacity > 0, "latest channel capacity must be positive");
    let (sender, receiver) = bounded(capacity);
    (
        LatestSender {
            sender,
            eviction_receiver: receiver.clone(),
            dropped: Arc::new(AtomicU64::new(0)),
        },
        receiver,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_discards_oldest_item() {
        let (sender, receiver) = latest_channel(2);
        sender.send_latest(1);
        sender.send_latest(2);
        sender.send_latest(3);

        assert_eq!(receiver.try_iter().collect::<Vec<_>>(), vec![2, 3]);
        assert_eq!(sender.dropped_count(), 1);
    }
}
