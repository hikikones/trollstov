use std::{collections::VecDeque, path::PathBuf, sync::mpsc};

use indexmap::IndexMap;

use audio::{AudioFile, AudioFileExtension, AudioFileReport, AudioRating};

use crate::*;

type AudioFileReceiver = mpsc::Receiver<Result<(AudioFile, AudioFileExtension), AudioFileReport>>;
type AudioWriteHandle = std::thread::JoinHandle<Result<(TrackId, AudioRating), AudioFileReport>>;

pub struct Database {
    music_dir: PathBuf,
    tracks: IndexMap<TrackId, Track>,
    sort: TrackSort,
    matcher: Matcher,
    buffer: String,
    receiver: Option<AudioFileReceiver>,
    write_handle: Option<AudioWriteHandle>,
    write_queue: VecDeque<(TrackId, AudioRating)>,
}

pub enum DatabaseEvent {
    Rating(TrackId),
    Error(AudioFileReport),
}

impl Database {
    pub fn new(music_dir: PathBuf) -> Self {
        Self {
            music_dir,
            tracks: IndexMap::new(),
            sort: TrackSort::default(),
            matcher: Matcher::new(),
            buffer: String::new(),
            receiver: None,
            write_handle: None,
            write_queue: VecDeque::new(),
        }
    }

    pub fn load(&mut self) {
        let (sender, receiver) = mpsc::channel();
        traverse_and_process_audio_files(self.music_dir.clone(), true, sender);
        self.receiver = Some(receiver);
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

    pub fn get_index_from_id(&self, id: TrackId) -> Option<usize> {
        self.tracks.get_index_of(&id)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, &Track)> {
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

    pub fn search(
        &mut self,
        keywords: &str,
        include_path: bool,
    ) -> impl Iterator<Item = (TrackId, u16)> {
        search(
            &mut self.matcher,
            &mut self.buffer,
            self.tracks.iter().map(|(id, track)| (*id, track)),
            keywords,
            include_path,
        )
    }

    pub fn write_rating(&mut self, id: TrackId, rating: AudioRating) {
        self.write_queue.push_back((id, rating));
    }

    pub fn random_id(&self) -> TrackId {
        TrackId(fastrand::u64(0..self.tracks.len() as u64))
    }

    fn start_write(&mut self, id: TrackId, rating: AudioRating) -> Option<AudioWriteHandle> {
        let Some(track) = self.tracks.get(&id) else {
            return None;
        };

        if track.rating() == rating {
            return None;
        }

        let path = track.path().to_path_buf();
        let extension = track.extension();
        let handle = std::thread::spawn(move || {
            let mut audio_file = AudioFile::read_from(path, extension)?;
            audio_file.write_rating(rating)?;
            Ok((id, rating))
        });
        Some(handle)
    }

    pub fn update(&mut self, mut on_event: impl FnMut(DatabaseEvent)) {
        // Receive processed audio files and convert to tracks
        if let Some(receiver) = self.receiver.as_ref() {
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
                                on_event(DatabaseEvent::Error(err));
                            }
                        }
                    }
                    Err(err) => match err {
                        mpsc::TryRecvError::Empty => {
                            break;
                        }
                        mpsc::TryRecvError::Disconnected => {
                            self.receiver = None;
                            break;
                        }
                    },
                }
            }
        }

        // Poll thread handle for finished tag writing
        match self.write_handle.as_ref() {
            Some(handle) => {
                if handle.is_finished() {
                    let handle = self.write_handle.take().unwrap();
                    match handle.join().unwrap() {
                        Ok((id, new_rating)) => {
                            if let Some(track) = self.tracks.get_mut(&id) {
                                track.set_rating(new_rating);
                                on_event(DatabaseEvent::Rating(id));
                            }
                        }
                        Err(err) => {
                            on_event(DatabaseEvent::Error(err));
                        }
                    }
                }
            }
            None => {
                self.write_handle = self
                    .write_queue
                    .pop_front()
                    .and_then(|(id, rating)| self.start_write(id, rating));
            }
        }
    }

    /// Gracefully shutdown by waiting for thread to finish writing the rating tag
    pub fn shutdown(mut self) {
        if let Some(handle) = self.write_handle.take() {
            let _ = handle.join().unwrap();
        }
    }
}

fn search<'a>(
    matcher: &mut Matcher,
    buffer: &mut String,
    tracks: impl Iterator<Item = (TrackId, &'a Track)>,
    keywords: &str,
    include_path: bool,
) -> impl Iterator<Item = (TrackId, u16)> {
    matcher.update(keywords);
    tracks.filter_map(move |(id, track)| {
        buffer.extend([track.artist(), " ", track.album(), " ", track.title()]);

        if include_path {
            buffer.extend([" ", track.path().to_string_lossy().as_ref()]);
        }

        let score = matcher.score(&buffer);
        buffer.clear();

        score.map(|score| (id, score))
    })
}

fn traverse_and_process_audio_files(
    root: PathBuf,
    follow_links: bool,
    sender: mpsc::Sender<Result<(AudioFile, AudioFileExtension), AudioFileReport>>,
) {
    std::thread::spawn(move || {
        walkdir::WalkDir::new(root)
            .follow_links(follow_links)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|file| {
                AudioFileExtension::from_path(file.path()).map(|ext| (file.into_path(), ext))
            })
            .for_each(|(path, extension)| {
                let audio_file =
                    AudioFile::read_from(path, extension).map(|audio_file| (audio_file, extension));
                let _ = sender.send(audio_file);
            });
    });
}

fn _traverse_and_process_audio_files_in_parallel(
    root: PathBuf,
    follow_links: bool,
    sender: mpsc::Sender<Result<(AudioFile, AudioFileExtension), AudioFileReport>>,
) {
    std::thread::spawn(move || {
        ignore::WalkBuilder::new(root)
            .follow_links(follow_links)
            .build_parallel()
            .run(|| {
                let sender = sender.clone();
                Box::new(move |result| {
                    if let Ok(dir_entry) = result {
                        if let Some(file_type) = dir_entry.file_type() {
                            if file_type.is_file() {
                                if let Some(extension) =
                                    AudioFileExtension::from_path(dir_entry.path())
                                {
                                    let audio_file =
                                        AudioFile::read_from(dir_entry.into_path(), extension)
                                            .map(|audio_file| (audio_file, extension));
                                    let _ = sender.send(audio_file);
                                }
                            }
                        }
                    }

                    ignore::WalkState::Continue
                })
            });
    });
}
