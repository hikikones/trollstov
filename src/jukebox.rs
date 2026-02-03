use std::{
    cmp::Ordering,
    collections::HashSet,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

use indexmap::IndexMap;
use rodio::decoder::Decoder;

use crate::{
    audio::*,
    events::{AppEvent, EventSender},
    pages::Log,
    utils,
};

type AudioFileReceiver = mpsc::Receiver<Result<(AudioFile, AudioFileExtension), AudioFileReport>>;
type AudioDecodeHandle = std::thread::JoinHandle<Result<Decoder<BufReader<File>>, AudioFileReport>>;
type AudioWriteHandle =
    std::thread::JoinHandle<Result<(TrackId, Option<AudioRating>), AudioFileReport>>;

pub struct Jukebox {
    music_dir: PathBuf,
    tracks: IndexMap<TrackId, Track>,
    sort: TrackSort,
    current: Option<TrackId>,
    queue: PlayQueue,
    state: PlayState,
    audio_file_receiver: Option<AudioFileReceiver>,
    audio_decode_handle: Option<(TrackId, AudioDecodeHandle)>,
    audio_write_handles: Vec<AudioWriteHandle>,
    faulty: HashSet<TrackId>,
    events: EventSender,
    sink: rodio::Sink,
    _stream: rodio::OutputStream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayState {
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    Track,
}

impl Jukebox {
    pub fn new(dir: impl AsRef<Path>, events: EventSender) -> Result<Self, rodio::StreamError> {
        let stream = rodio::OutputStreamBuilder::open_default_stream()?;
        let sink = rodio::Sink::connect_new(stream.mixer());
        sink.pause();

        Ok(Self {
            music_dir: dir.as_ref().to_path_buf(),
            tracks: IndexMap::new(),
            sort: TrackSort::default(),
            current: None,
            queue: PlayQueue::new(),
            state: PlayState::Stop,
            audio_file_receiver: None,
            audio_decode_handle: None,
            audio_write_handles: Vec::new(),
            faulty: HashSet::new(),
            events,
            sink,
            _stream: stream,
        })
    }

    pub fn load(&mut self) {
        let (sender, receiver) = mpsc::channel();
        traverse_and_process_audio_files(self.music_dir.as_path(), true, sender);
        self.audio_file_receiver = Some(receiver);
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn get(&self, id: TrackId) -> Option<&Track> {
        self.tracks.get(&id)
    }

    pub fn get_mut(&mut self, id: TrackId) -> Option<&mut Track> {
        self.tracks.get_mut(&id)
    }

    pub fn get_id_from_index(&self, i: usize) -> Option<TrackId> {
        self.tracks.keys().nth(i).copied()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, &Track)> + DoubleEndedIterator {
        self.tracks.iter().map(|(id, track)| (*id, track))
    }

    pub const fn get_sort(&self) -> TrackSort {
        self.sort
    }

    pub fn sort(&mut self, sort: TrackSort) {
        self.tracks
            .sort_unstable_by(|_, track1, _, track2| sort.cmp(track1, track2));
        self.sort = sort;
    }

    pub const fn current_track_id(&self) -> Option<TrackId> {
        self.current
    }

    pub fn current_track_pos(&self) -> Duration {
        self.sink.get_pos()
    }

    pub fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn queue_iter(&self) -> impl ExactSizeIterator<Item = TrackId> {
        self.queue.iter().copied()
    }

    pub fn enqueue(&mut self, id: TrackId) {
        self.queue.enqueue(id);
    }

    pub fn enqueue_next(&mut self, id: TrackId) {
        self.queue.enqueue_next(id);
    }

    pub fn update(&mut self) {
        self.receive_audio_files();

        let mut render = false;

        // Poll thread handle for audio decoding
        if let Some((_, handle)) = self.audio_decode_handle.as_ref() {
            if handle.is_finished() {
                render = true;
                let (id, handle) = self.audio_decode_handle.take().unwrap();
                match handle.join().unwrap() {
                    Ok(decoded_audio) => {
                        self.sink.clear();
                        self.sink.append(decoded_audio);
                        if self.state != PlayState::Pause {
                            self.state = PlayState::Play;
                            self.sink.play();
                        }
                        self.current = Some(id);
                    }
                    Err(err) => {
                        let log = Log::new(err);
                        self.events.send(AppEvent::Log(log));
                        self.faulty.insert(id);
                        match self.state {
                            PlayState::Play | PlayState::Next => {
                                self.play_next();
                            }
                            PlayState::Previous => {
                                self.play_previous();
                            }
                            PlayState::Track => {
                                self.state = PlayState::Play;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Poll thread handles for finished tag writing
        // TODO: Write handles should be polled one by one.
        // Right now, multiple threads _could_ be writing to the same file.
        for _ in 0..self.audio_write_handles.len() {
            let handle = self.audio_write_handles.pop().unwrap();

            if !handle.is_finished() {
                self.audio_write_handles.push(handle);
                continue;
            }

            match handle.join().unwrap() {
                Ok((id, new_rating)) => {
                    let track = self.get_mut(id).unwrap();
                    track.set_rating(new_rating);
                    render = true;
                }
                Err(err) => {
                    let log = Log::new(err);
                    self.events.send(AppEvent::Log(log));
                }
            }
        }

        // Play next when empty and idle
        let is_finished = self.sink.empty() && !self.sink.is_paused();
        if is_finished {
            match self.state {
                PlayState::Play => {
                    self.play_next();
                }
                PlayState::Next | PlayState::Previous | PlayState::Track => {
                    self.state = PlayState::Play;
                }
                _ => {}
            }
        }

        if render {
            self.events.send(AppEvent::Render);
        }
    }

    pub fn play_track(&mut self, id: TrackId) {
        // TODO: If new track is same as current, simply rewind.
        let id = self.queue.enqueue_next(id).next().unwrap();
        self.state = PlayState::Track;
        self.start_play(id);
    }

    pub fn play(&mut self) {
        self.state = PlayState::Play;
        self.sink.play();
    }

    pub fn pause(&mut self) {
        self.state = PlayState::Pause;
        self.sink.pause();
    }

    pub fn pause_or_play(&mut self) {
        if self.sink.is_paused() {
            self.play();
        } else {
            self.pause();
        }
    }

    pub fn stop(&mut self) {
        // TODO: Should stop also clear queue and history?
        if self.current.is_some() {
            self.events.send(AppEvent::UpdateAndRender);
        }

        self.sink.clear();
        self.current = None;
        self.state = PlayState::Stop;
        self.audio_decode_handle = None;
    }

    pub fn play_next(&mut self) {
        let mut next = self.queue.current_or_next();

        while let Some(id) = next {
            // TODO: Fix when next is actually current.
            // This happens when going backwards/previous but all tracks errors out.
            // Then going forward again will replay the current track.
            if self.faulty.contains(&id) {
                next = self.queue.next();
            } else {
                break;
            }
        }

        match next {
            Some(id) => {
                self.state = PlayState::Next;
                self.start_play(id);
            }
            None => {
                if self.tracks.is_empty() {
                    return;
                }

                let id = self
                    .queue
                    .enqueue_next(TrackId(fastrand::u64(0..self.tracks.len() as u64)))
                    .next()
                    .unwrap();
                self.state = PlayState::Next;
                self.start_play(id);
            }
        }
    }

    pub fn play_previous(&mut self) {
        if let Some(id) = self.queue.previous() {
            self.state = PlayState::Previous;
            self.start_play(id);
        }
    }

    pub fn fast_forward_by(&mut self, duration: Duration) {
        if self.sink.empty() {
            return;
        }

        let _ = self.sink.try_seek(self.sink.get_pos() + duration);
    }

    pub fn set_rating(&mut self, id: TrackId, rating: AudioRating) {
        let handle = self.write_rating(id, rating);
        self.audio_write_handles.push(handle);
    }

    fn start_play(&mut self, id: TrackId) {
        self.audio_decode_handle = Some((id, self.decode_audio(id)));
        self.events.send(AppEvent::UpdateAndRender);
    }

    fn decode_audio(&mut self, id: TrackId) -> AudioDecodeHandle {
        let track = self.tracks.get(&id).unwrap();
        let path = track.path().to_path_buf();

        std::thread::spawn(move || {
            let file = File::open(&path).map_err(|err| {
                AudioFileReport::new(format!(
                    "Could not open audio file {} due to {}",
                    path.display(),
                    err
                ))
            })?;
            let buffer = BufReader::new(file);
            let source = Decoder::new(buffer).map_err(|err| {
                AudioFileReport::new(format!(
                    "Could not decode audio file {} due to {}",
                    path.display(),
                    err
                ))
            })?;
            Ok(source)
        })
    }

    fn write_rating(&mut self, id: TrackId, rating: AudioRating) -> AudioWriteHandle {
        let track = self.tracks.get(&id).unwrap();
        let path = track.path().to_path_buf();
        let extension = track.extension();
        let current_rating = track.rating();

        let new_rating = match current_rating {
            Some(current_rating) => {
                if current_rating != rating {
                    // Replace rating when they differ
                    Some(rating)
                } else {
                    // Remove rating when they are the same
                    None
                }
            }
            None => {
                // Insert new rating
                Some(rating)
            }
        };

        std::thread::spawn(move || {
            let mut audio_file = AudioFile::read_from(path, extension)?;
            audio_file.write_rating(new_rating)?;
            Ok((id, new_rating))
        })
    }

    fn receive_audio_files(&mut self) {
        let Some(receiver) = self.audio_file_receiver.as_ref() else {
            return;
        };

        // Receive processed audio files and convert to tracks
        loop {
            match receiver.try_recv() {
                Ok(audio_file_res) => {
                    let track_res = audio_file_res.and_then(|(audio_file, extension)| {
                        let track = Track::new(
                            audio_file.metadata()?,
                            audio_file.properties(),
                            audio_file.path().to_path_buf(),
                            extension,
                        );
                        Ok(track)
                    });
                    match track_res {
                        Ok(track) => {
                            let last_id = self.tracks.len() as u64;
                            self.tracks.insert_sorted_by(
                                TrackId(last_id),
                                track,
                                |_, track1, _, track2| self.sort.cmp(track1, track2),
                            );
                        }
                        Err(err) => {
                            let log = Log::new(err);
                            self.events.send(AppEvent::Log(log));
                        }
                    }
                }
                Err(err) => match err {
                    mpsc::TryRecvError::Empty => {
                        break;
                    }
                    mpsc::TryRecvError::Disconnected => {
                        self.audio_file_receiver = None;
                        break;
                    }
                },
            }
        }
    }

    pub fn shutdown(self) {
        // Gracefully shutdown by waiting for threads to finish writing tag
        for handle in self.audio_write_handles {
            let _ = handle.join().unwrap();
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(u64);

#[derive(Debug)]
pub struct Track {
    metadata: AudioMetadata,
    properties: AudioProperties,
    path: PathBuf,
    extension: AudioFileExtension,
    duration_display: String,
}

impl Track {
    fn new(
        metadata: AudioMetadata,
        properties: AudioProperties,
        path: PathBuf,
        extension: AudioFileExtension,
    ) -> Self {
        let duration_display = utils::format_duration(properties.duration());

        Self {
            metadata,
            properties,
            path,
            extension,
            duration_display,
        }
    }

    pub const fn title(&self) -> &str {
        self.metadata.title()
    }

    pub const fn artist(&self) -> &str {
        self.metadata.artist()
    }

    pub const fn album(&self) -> &str {
        self.metadata.album()
    }

    pub const fn rating(&self) -> Option<AudioRating> {
        self.metadata.rating()
    }

    pub const fn rating_display(&self) -> &str {
        match self.metadata.rating() {
            Some(rating) => match rating {
                AudioRating::Awful => "★",
                AudioRating::Bad => "★★",
                AudioRating::Ok => "★★★",
                AudioRating::Good => "★★★★",
                AudioRating::Amazing => "★★★★★",
            },
            None => "",
        }
    }

    pub const fn set_rating(&mut self, rating: Option<AudioRating>) {
        self.metadata.set_rating(rating);
    }

    pub const fn duration(&self) -> Duration {
        self.properties.duration()
    }

    pub const fn duration_display(&self) -> &str {
        self.duration_display.as_str()
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub const fn extension(&self) -> AudioFileExtension {
        self.extension
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TrackSort {
    TitleAscending,
    TitleDescending,
    ArtistAscending,
    ArtistDescending,
    #[default]
    AlbumAscending,
    AlbumDescending,
    TimeAscending,
    TimeDescending,
    RatingAscending,
    RatingDescending,
}

impl TrackSort {
    pub const fn next(self) -> Self {
        match self {
            Self::TitleAscending => Self::TitleDescending,
            Self::TitleDescending => Self::ArtistAscending,
            Self::ArtistAscending => Self::ArtistDescending,
            Self::ArtistDescending => Self::AlbumAscending,
            Self::AlbumAscending => Self::AlbumDescending,
            Self::AlbumDescending => Self::TimeAscending,
            Self::TimeAscending => Self::TimeDescending,
            Self::TimeDescending => Self::RatingAscending,
            Self::RatingAscending => Self::RatingDescending,
            Self::RatingDescending => Self::TitleAscending,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::TitleAscending => Self::RatingDescending,
            Self::TitleDescending => Self::TitleAscending,
            Self::ArtistAscending => Self::TitleDescending,
            Self::ArtistDescending => Self::ArtistAscending,
            Self::AlbumAscending => Self::ArtistDescending,
            Self::AlbumDescending => Self::AlbumAscending,
            Self::TimeAscending => Self::AlbumDescending,
            Self::TimeDescending => Self::TimeAscending,
            Self::RatingAscending => Self::TimeDescending,
            Self::RatingDescending => Self::RatingAscending,
        }
    }

    fn cmp(self, t1: &Track, t2: &Track) -> Ordering {
        match self {
            Self::TitleAscending => t1.title().cmp(t2.title()),
            Self::TitleDescending => t2.title().cmp(t1.title()),
            Self::ArtistAscending => t1.artist().cmp(t2.artist()),
            Self::ArtistDescending => t2.artist().cmp(t1.artist()),
            Self::AlbumAscending => t1.album().cmp(t2.album()),
            Self::AlbumDescending => t2.album().cmp(t1.album()),
            Self::TimeAscending => t1.duration().cmp(&t2.duration()),
            Self::TimeDescending => t2.duration().cmp(&t1.duration()),
            Self::RatingAscending => {
                let r1 = t1.rating().map(|r| r as u8).unwrap_or(0);
                let r2 = t2.rating().map(|r| r as u8).unwrap_or(0);
                r1.cmp(&r2)
            }
            Self::RatingDescending => {
                let r1 = t1.rating().map(|r| r as u8).unwrap_or(0);
                let r2 = t2.rating().map(|r| r as u8).unwrap_or(0);
                r2.cmp(&r1)
            }
        }
    }
}

// TODO: max length?
struct PlayQueue {
    list: Vec<TrackId>,
    index: Option<usize>,
}

impl PlayQueue {
    const fn new() -> Self {
        Self {
            list: Vec::new(),
            index: None,
        }
    }

    const fn len(&self) -> usize {
        self.list.len()
    }

    const fn queue_len(&self) -> usize {
        match self.index {
            Some(index) => (self.list.len() - index).saturating_sub(1),
            None => self.list.len(),
        }
    }

    const fn history_len(&self) -> usize {
        match self.index {
            Some(index) => index,
            None => 0,
        }
    }

    const fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    fn current(&self) -> Option<(TrackId, usize)> {
        self.index
            .and_then(|i| self.list.get(i).copied().map(|id| (id, i)))
    }

    fn iter(&self) -> std::slice::Iter<'_, TrackId> {
        self.list.iter()
    }

    fn enqueue(&mut self, id: TrackId) {
        self.list.push(id);
    }

    fn enqueue_next(&mut self, id: TrackId) -> &mut Self {
        let insert_index = self.index.map(|i| i + 1).unwrap_or_default();
        self.list.insert(insert_index, id);
        self
    }

    fn current_or_next(&mut self) -> Option<TrackId> {
        self.current().map(|(id, _)| id).or_else(|| self.next())
    }

    fn next(&mut self) -> Option<TrackId> {
        match self.index {
            Some(mut index) => {
                let old_index = index;
                let max_index = self.len().saturating_sub(1);
                index = usize::min(index + 1, max_index);

                if index != old_index {
                    self.index = Some(index);
                    self.list.get(index).copied()
                } else {
                    None
                }
            }
            None => {
                if self.list.is_empty() {
                    None
                } else {
                    self.index = Some(0);
                    self.list.first().copied()
                }
            }
        }
    }

    fn previous(&mut self) -> Option<TrackId> {
        match self.index {
            Some(mut index) => {
                let old_index = index;
                index = index.saturating_sub(1);

                if index != old_index {
                    self.index = Some(index);
                    self.list.get(index).copied()
                } else {
                    None
                }
            }
            None => None,
        }
    }

    fn clear(&mut self) {
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
        assert_eq!(queue.next(), Some(TrackId(0)));
        assert_eq!(queue.current(), Some((TrackId(0), 0)));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), TRACKS_LEN - 1);
        assert_eq!(queue.history_len(), 0);

        assert_eq!(queue.next(), Some(TrackId(1)));
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
        assert_eq!(queue.previous(), Some(TrackId(0)));
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
