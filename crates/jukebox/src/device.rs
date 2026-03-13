use std::time::Duration;

pub struct AudioDevice {
    sink: rodio::Sink,
    _stream: rodio::OutputStream,
}

impl AudioDevice {
    pub fn new() -> Result<Self, rodio::StreamError> {
        let mut stream = rodio::OutputStreamBuilder::open_default_stream()?;
        let sink = rodio::Sink::connect_new(stream.mixer());

        stream.log_on_drop(false);
        sink.pause();

        Ok(Self {
            sink,
            _stream: stream,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.sink.empty()
    }

    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    pub fn volume(&self) -> f32 {
        self.sink.volume()
    }

    pub fn set_volume(&self, value: f32) {
        self.sink.set_volume(value);
    }

    pub fn position(&self) -> Duration {
        self.sink.get_pos()
    }

    pub fn append(&self, source: impl rodio::Source + Send + 'static) {
        self.sink.append(source);
    }

    pub fn play(&self) {
        self.sink.play();
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn clear(&self) {
        self.sink.clear();
    }

    pub fn seek(&self, pos: Duration) {
        let _ = self.sink.try_seek(pos);
    }
}
