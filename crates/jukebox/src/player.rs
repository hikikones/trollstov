use std::time::Duration;

pub struct AudioPlayer {
    player: rodio::Player,
    _sink: rodio::MixerDeviceSink,
}

impl AudioPlayer {
    pub fn new() -> Result<Self, rodio::DeviceSinkError> {
        let mut sink = rodio::DeviceSinkBuilder::open_default_sink()?;
        let player = rodio::Player::connect_new(sink.mixer());

        sink.log_on_drop(false);
        player.pause();

        Ok(Self {
            player,
            _sink: sink,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.player.empty()
    }

    pub fn is_paused(&self) -> bool {
        self.player.is_paused()
    }

    pub fn volume(&self) -> f32 {
        self.player.volume()
    }

    pub fn set_volume(&self, value: f32) {
        self.player.set_volume(value);
    }

    pub fn position(&self) -> Duration {
        self.player.get_pos()
    }

    pub fn append(&self, source: impl rodio::Source + Send + 'static) {
        self.player.append(source);
    }

    pub fn play(&self) {
        self.player.play();
    }

    pub fn pause(&self) {
        self.player.pause();
    }

    pub fn clear(&self) {
        self.player.clear();
    }

    pub fn seek(&self, pos: Duration) {
        let _ = self.player.try_seek(pos);
    }
}
