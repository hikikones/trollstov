use super::TrackId;

// TODO: max length?
pub(super) struct PlayQueue {
    list: Vec<TrackId>,
    index: Option<QueueIndex>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueueIndex(usize);

impl PlayQueue {
    pub(super) const fn new() -> Self {
        Self {
            list: Vec::new(),
            index: None,
        }
    }

    pub(super) const fn len(&self) -> usize {
        self.list.len()
    }

    pub(super) const fn queue_len(&self) -> usize {
        match self.index {
            Some(index) => (self.list.len() - index.0).saturating_sub(1),
            None => self.list.len(),
        }
    }

    pub(super) const fn history_len(&self) -> usize {
        match self.index {
            Some(index) => index.0,
            None => 0,
        }
    }

    pub(super) const fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub(super) fn current(&self) -> Option<(TrackId, QueueIndex)> {
        self.index
            .and_then(|i| self.list.get(i.0).copied().map(|id| (id, i)))
    }

    pub(super) fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, QueueIndex)> {
        self.list
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, QueueIndex(i)))
    }

    pub(super) fn enqueue(&mut self, id: TrackId) -> &mut Self {
        self.list.push(id);
        self
    }

    pub(super) fn enqueue_next(&mut self, id: TrackId) -> &mut Self {
        let insert_index = self.index.map(|i| i.0 + 1).unwrap_or_default();
        self.list.insert(insert_index, id);
        self
    }

    pub(super) fn current_or_next(&mut self) -> Option<(TrackId, QueueIndex)> {
        self.current().or_else(|| self.next())
    }

    pub(super) fn next(&mut self) -> Option<(TrackId, QueueIndex)> {
        match self.index {
            Some(QueueIndex(mut index)) => {
                let old_index = index;
                let max_index = self.len().saturating_sub(1);
                index = usize::min(index + 1, max_index);

                if index != old_index {
                    self.index = Some(QueueIndex(index));
                    self.list
                        .get(index)
                        .copied()
                        .map(|id| (id, QueueIndex(index)))
                } else {
                    None
                }
            }
            None => {
                if self.list.is_empty() {
                    None
                } else {
                    self.index = Some(QueueIndex(0));
                    self.list.first().copied().map(|id| (id, QueueIndex(0)))
                }
            }
        }
    }

    pub(super) fn previous(&mut self) -> Option<(TrackId, QueueIndex)> {
        match self.index {
            Some(QueueIndex(mut index)) => {
                let old_index = index;
                index = index.saturating_sub(1);

                if index != old_index {
                    self.index = Some(QueueIndex(index));
                    self.list
                        .get(index)
                        .copied()
                        .map(|id| (id, QueueIndex(index)))
                } else {
                    None
                }
            }
            None => None,
        }
    }

    pub(super) fn clear(&mut self) {
        self.list.clear();
        self.index = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_queue() {
        const TRACKS_LEN: usize = 2;
        let mut queue = PlayQueue::new();

        for i in 0..TRACKS_LEN {
            queue.enqueue(TrackId(i as u64));
        }

        assert_eq!(queue.current(), None);
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), TRACKS_LEN);
        assert_eq!(queue.history_len(), 0);

        // Next
        assert_eq!(queue.next(), Some((TrackId(0), QueueIndex(0))));
        assert_eq!(queue.current(), Some((TrackId(0), QueueIndex(0))));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), TRACKS_LEN - 1);
        assert_eq!(queue.history_len(), 0);

        assert_eq!(queue.next(), Some((TrackId(1), QueueIndex(1))));
        assert_eq!(queue.current(), Some((TrackId(1), QueueIndex(1))));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), TRACKS_LEN - 2);
        assert_eq!(queue.history_len(), 1);

        assert_eq!(queue.next(), None);
        assert_eq!(queue.current(), Some((TrackId(1), QueueIndex(1))));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), 0);
        assert_eq!(queue.history_len(), 1);

        // Previous
        assert_eq!(queue.previous(), Some((TrackId(0), QueueIndex(0))));
        assert_eq!(queue.current(), Some((TrackId(0), QueueIndex(0))));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), 1);
        assert_eq!(queue.history_len(), 0);

        assert_eq!(queue.previous(), None);
        assert_eq!(queue.current(), Some((TrackId(0), QueueIndex(0))));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), 1);
        assert_eq!(queue.history_len(), 0);
    }
}
