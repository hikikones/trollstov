use database::TrackId;

// TODO: Max length? Drain from history.
// TODO: Move up/down.

pub(crate) struct PlayQueue {
    list: Vec<TrackId>,
    index: Option<usize>,
}

impl PlayQueue {
    pub(crate) const fn new() -> Self {
        Self {
            list: Vec::new(),
            index: None,
        }
    }

    pub(crate) const fn len(&self) -> usize {
        self.list.len()
    }

    pub(crate) const fn queue_len(&self) -> usize {
        match self.index {
            Some(index) => (self.list.len() - index).saturating_sub(1),
            None => self.list.len(),
        }
    }

    pub(crate) const fn history_len(&self) -> usize {
        match self.index {
            Some(index) => index,
            None => 0,
        }
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub(crate) fn get(&self, index: usize) -> Option<TrackId> {
        self.list.get(index).copied()
    }

    pub(crate) fn set_index(&mut self, index: usize) -> Option<TrackId> {
        match self.list.get(index).copied() {
            Some(id) => {
                self.index = Some(index);
                Some(id)
            }
            None => None,
        }
    }

    pub(crate) fn current(&self) -> Option<(TrackId, usize)> {
        self.index
            .and_then(|i| self.list.get(i).copied().map(|id| (id, i)))
    }

    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, usize)> {
        self.list.iter().enumerate().map(|(i, id)| (*id, i))
    }

    pub(crate) fn enqueue(&mut self, id: TrackId) -> &mut Self {
        self.list.push(id);
        self
    }

    pub(crate) fn enqueue_next(&mut self, id: TrackId) -> &mut Self {
        let insert_index = self.index.map(|i| i + 1).unwrap_or_default();
        self.list.insert(insert_index, id);
        self
    }

    pub(crate) fn current_or_next(&mut self) -> Option<(TrackId, usize)> {
        self.current().or_else(|| self.next())
    }

    pub(crate) fn next(&mut self) -> Option<(TrackId, usize)> {
        match self.index {
            Some(mut index) => {
                let old_index = index;
                let max_index = self.len().saturating_sub(1);
                index = usize::min(index + 1, max_index);

                if index != old_index {
                    self.index = Some(index);
                    self.list.get(index).copied().map(|id| (id, index))
                } else {
                    None
                }
            }
            None => {
                if self.list.is_empty() {
                    None
                } else {
                    self.index = Some(0);
                    self.list.first().copied().map(|id| (id, 0))
                }
            }
        }
    }

    pub(crate) fn previous(&mut self) -> Option<(TrackId, usize)> {
        match self.index {
            Some(mut index) => {
                let old_index = index;
                index = index.saturating_sub(1);

                if index != old_index {
                    self.index = Some(index);
                    self.list.get(index).copied().map(|id| (id, index))
                } else {
                    None
                }
            }
            None => None,
        }
    }

    pub(crate) fn shuffle(&mut self, start: usize) {
        let end = self.list.len();
        if start >= end {
            return;
        }

        for i in start..end {
            let random = fastrand::usize(start..end);
            self.list.swap(i, random);
        }
    }

    pub(crate) fn remove(&mut self, index: usize, keep_current: bool) -> Option<TrackId> {
        if index >= self.len() {
            return None;
        }

        let Some(current) = self.index else {
            return Some(self.list.remove(index));
        };

        if index == current && keep_current {
            return None;
        }

        let id = self.list.remove(index);
        self.index = self.index.and_then(|current| {
            if self.list.is_empty() {
                None
            } else if index < current {
                Some(current - 1)
            } else {
                Some(current.min(self.list.len().saturating_sub(1)))
            }
        });
        Some(id)
    }

    pub(crate) fn remove_range(&mut self, start: usize, end: usize, keep_current: bool) -> bool {
        let end = end.min(self.list.len().saturating_sub(1));

        if start > end {
            return false;
        }

        let Some(current) = self.index else {
            self.list.drain(start..=end);
            return true;
        };

        let id = self.list[current];

        let mut offset = 0;
        for index in self
            .list
            .drain(start..=end)
            .enumerate()
            .map(|(i, _)| start + i)
        {
            if index < current {
                offset += 1;
            }
        }

        let contains_current = current >= start && current <= end;
        let keep_current = contains_current && keep_current;
        if self.list.is_empty() {
            if keep_current {
                self.list.push(id);
                self.index = Some(0);
            } else {
                self.index = None;
            }
        } else {
            if keep_current {
                let index = (current - offset).min(self.list.len());
                self.list.insert(index, id);
                self.index = Some(index);
            } else {
                let index = (current - offset).min(self.list.len().saturating_sub(1));
                self.index = Some(index);
            }
        }

        true
    }

    pub(crate) const fn reset(&mut self) {
        self.index = None;
    }

    pub(crate) fn clear(&mut self) {
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
        assert_eq!(queue.next(), Some((TrackId(0), 0)));
        assert_eq!(queue.current(), Some((TrackId(0), 0)));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), TRACKS_LEN - 1);
        assert_eq!(queue.history_len(), 0);

        assert_eq!(queue.next(), Some((TrackId(1), 1)));
        assert_eq!(queue.current(), Some((TrackId(1), 1)));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), TRACKS_LEN - 2);
        assert_eq!(queue.history_len(), 1);

        assert_eq!(queue.next(), None);
        assert_eq!(queue.current(), Some((TrackId(1), 1)));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), 0);
        assert_eq!(queue.history_len(), 1);

        // Previous
        assert_eq!(queue.previous(), Some((TrackId(0), 0)));
        assert_eq!(queue.current(), Some((TrackId(0), 0)));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), 1);
        assert_eq!(queue.history_len(), 0);

        assert_eq!(queue.previous(), None);
        assert_eq!(queue.current(), Some((TrackId(0), 0)));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), 1);
        assert_eq!(queue.history_len(), 0);
    }
}
