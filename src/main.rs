#![windows_subsystem = "windows"]

use anyhow::Result;
use druid::{
    widget::{Align, Button, Flex, Label, List, Slider},
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Size,
    TimerToken, UpdateCtx,
};
use druid::{AppLauncher, Data, Lens, Widget, WidgetExt, WindowDesc};
use std::time::Duration;
use std::{sync::Arc, time::Instant};
use unicode_segmentation::UnicodeSegmentation;

const HOSTNAME: &str = "http://localhost:81/";
const TIMER_INTERVAL: Duration = Duration::from_millis(100);
const DEFAULT_VOLUME: f64 = 0.30f64;
const WINDOW_WIDTH: f64 = 650f64;
const WINDOW_HEIGHT: f64 = 350f64;

struct TimerWidget {
    timer_id: TimerToken,
}

impl Widget<DruidState> for TimerWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut DruidState, _env: &Env) {
        match event {
            Event::WindowConnected => {
                // Start the timer when the application launches
                self.timer_id = ctx.request_timer(TIMER_INTERVAL);
                // Start first Song
                data.dl_play().unwrap_or(());
            }
            Event::Timer(id) => {
                if *id == self.timer_id {
                    self.timer_id = ctx.request_timer(TIMER_INTERVAL);
                    timer_tick(ctx, data);
                }
            }
            _ => (),
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &DruidState,
        _env: &Env,
    ) {
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx,
        _old_data: &DruidState,
        _data: &DruidState,
        _env: &Env,
    ) {
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &DruidState,
        _env: &Env,
    ) -> Size {
        bc.constrain((0.0, 0.0))
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _data: &DruidState, _env: &Env) {}
}

fn timer_tick(_ctx: &mut EventCtx, data: &mut DruidState) {
    if let Some(sink) = data.sink.as_ref() {
        // check for autoplay, set volume, advance playtime display
        // look if pause button label needs to be updated
        if sink.empty() {
            println!("NEUES LIED!");
            data.dl_play().unwrap_or(());
        } else {
            sink.set_volume(data.corrected_volume());
            if !sink.is_paused() {
                let now = Instant::now();
                let delta = now - data.last_timestamp;
                data.playtime += delta.as_millis();
                data.last_timestamp = now;
                data.paused = false;
            } else {
                data.paused = true;
            }
        }

        // look if song is to be deleted
        for i in 0..data.items.len() {
            if data.items[i].skip {
                drop(data.drop_song(i));
                data.queue_song(SongData::fetch_random_song().unwrap())
            }
        }
    }
}

#[derive(Clone, Data, Lens)]
struct DruidState {
    handle: Arc<rodio::OutputStreamHandle>,
    sink: Option<Arc<rodio::Sink>>,
    volume: f64,
    #[data(ignore)]
    last_timestamp: Instant,
    playtime: u128,
    items: druid::im::Vector<SongData>,
    current_song: SongData,
    paused: bool,
}

fn main() -> Result<()> {
    let main_window = WindowDesc::new(ui_builder)
        .with_min_size((WINDOW_WIDTH, WINDOW_HEIGHT))
        .window_size((WINDOW_WIDTH, WINDOW_HEIGHT))
        .title("Rust PLAY");

    let (stream, handle) = rodio::OutputStream::try_default()?;

    let mut state = DruidState {
        handle: Arc::new(handle),
        sink: None,
        volume: DEFAULT_VOLUME,
        last_timestamp: Instant::now(),
        playtime: 0,
        items: Default::default(),
        current_song: SongData::default(),
        paused: false,
    };

    for _ in 0..5 {
        state.queue_song(SongData::fetch_random_song()?);
    }

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(state)?;

    // Stream darf erst hier gedroppt werden, sonst zeigt das Handle ins nichts -> Sound stoppt.
    drop(stream);

    Ok(())
}

fn ui_builder() -> impl Widget<DruidState> {
    let btn_skip = Button::new("⏭")
        .on_click(|_ctx, data: &mut DruidState, _env| data.dl_play().unwrap_or(()))
        .padding(5.0);

    let btn_upvote = Button::new(|data: &DruidState, _: &Env| {
        if !data.current_song.updooted {
            "👍".to_string()
        } else {
            "👍✓".to_string()
        }
    })
    .on_click(|_: &mut EventCtx, data: &mut DruidState, _: &Env| {
        data.current_song.updoot().unwrap_or_default()
    });

    let btn_downvote =
        Button::new("👎").on_click(|_: &mut EventCtx, data: &mut DruidState, _: &Env| {
            data.current_song.downdoot().unwrap_or_default();
            data.dl_play().unwrap_or(());
        });

    let btn_pauseplay = Button::new(|data: &DruidState, _: &Env| {
        if data.paused {
            "⏵".to_string()
        } else {
            "⏸︎".to_string()
        }
    })
    .on_click(|_ctx, data: &mut DruidState, _env| data.toggle_pause());

    let timer1 = TimerWidget {
        timer_id: TimerToken::INVALID,
    };

    let songnamelabel: Align<DruidState> = Label::new(|data: &DruidState, _: &_| {
        format!(
            "Playing: {} - {} <{}>",
            data.current_song.title, data.current_song.artist, data.current_song.rating
        )
    })
    .padding(5.0)
    .center();

    let progresslabel: Align<DruidState> = Label::new(|data: &DruidState, _env: &_| {
        format!(
            "{} / {}",
            format_songlength((data.playtime / 1000) as u64),
            data.current_song.playtime
        )
    })
    .padding(5.0)
    .center();

    let volumelabel: Align<DruidState> =
        Label::new(|data: &DruidState, _env: &_| format!("Volume: {:.2}", data.volume))
            .padding(5.0)
            .center();

    let volumeslider = Slider::new().lens(DruidState::volume);

    let songqueue = List::new(build_song_widget).lens(DruidState::items);

    let row1 = Flex::row()
        .with_child(btn_pauseplay)
        .with_child(btn_skip)
        .with_child(btn_upvote)
        .with_child(btn_downvote);

    Flex::column()
        .with_child(row1)
        .with_child(timer1)
        .with_child(songnamelabel)
        .with_child(progresslabel)
        .with_child(volumelabel)
        .with_child(volumeslider)
        .with_child(songqueue)
}

fn format_songlength(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    if mins >= 60 {
        let hours = mins / 60;
        let mins = mins / 60;
        format!("{}:{:0>2}:{:0>2}", hours, mins, secs)
    } else {
        format!("{:0>1}:{:0>2}", mins, secs)
    }
}

impl DruidState {
    fn dl(&mut self) -> Result<Vec<u8>> {
        let id = &self.current_song.id;
        Ok(
            reqwest::blocking::get(&format!("{}{}{}", HOSTNAME, "songs/", id))?
                .bytes()?
                .to_vec(),
        )
    }

    fn dl_play(&mut self) -> Result<()> {
        self.current_song = self.drop_song(0)?;
        let song = self.dl()?;
        self.play(song)?;
        self.queue_song(SongData::fetch_random_song()?);
        Ok(())
    }

    /// Corrects for dumb brain.
    fn corrected_volume(&self) -> f32 {
        (self.volume * self.volume * self.volume) as f32 // ^2 or ^3? hmmm...
    }

    fn toggle_pause(&mut self) {
        if let Some(ref sink) = self.sink {
            if sink.is_paused() {
                sink.play();
                self.last_timestamp = Instant::now();
            } else {
                sink.pause()
            }
        }
    }

    fn drop_song(&mut self, index: usize) -> Result<SongData> {
        if index < self.items.len() {
            let out = self.items.remove(index);
            Ok(out)
        } else {
            Err(anyhow::anyhow!("invalid song index!"))
        }
    }

    fn queue_song(&mut self, song: SongData) {
        self.items.push_back(song);
    }

    /// Kill old sink and create a new one with the handle from the DruidState.
    /// Play the local sound file.
    fn play(&mut self, song: Vec<u8>) -> Result<()> {
        // Create new sink
        let sink;
        if let Some(s) = self.sink.as_ref() {
            s.stop();
            self.sink = None;
        }
        self.sink = Some(Arc::new(rodio::Sink::try_new(&self.handle)?));

        // Unwrap als Assert. Es muss eine neue Sink geben.
        sink = self.sink.as_ref().unwrap();
        if !sink.empty() {
            sink.stop();
        }

        sink.set_volume(self.corrected_volume());

        let cursor = std::io::Cursor::new(song);
        let decode = rodio::Decoder::new_mp3(cursor)?;

        sink.append(decode);

        self.last_timestamp = Instant::now();
        self.playtime = 0;
        Ok(())
    }
}

/// Einzelner Song
#[derive(Clone, Data, Lens, Default)]
struct SongData {
    id: String,
    title: String,
    artist: String,
    album: String,
    playtime: String,
    rating: u32,
    skip: bool,
    updooted: bool,
    updoot_sync_marker: bool,
}

impl SongData {
    fn fetch_random_song() -> Result<SongData> {
        let mut result = SongData::default();

        let id = reqwest::blocking::get(&format!("{}{}", HOSTNAME, "random_id"))?.text()?;

        let songdata =
            reqwest::blocking::get(&format!("{}{}{}", HOSTNAME, "songdata/", id))?.text()?;
        let songdata = json::parse(&songdata)?;
        let mut title = songdata["songname"].to_string();
        if title.is_empty() {
            title = songdata["filename"].to_string()
        };

        result.id = id;
        result.title = title;
        result.artist = songdata["artist"].to_string();
        result.album = songdata["album"].to_string();
        result.playtime = songdata["length"].to_string();
        result.rating = songdata["rating"].to_string().parse().unwrap_or_default();
        result.skip = false;

        Ok(result)
    }

    fn updoot(&mut self) -> Result<()> {
        if !self.updooted {
            reqwest::blocking::get(&format!("{}{}{}", HOSTNAME, "upvote/", self.id))?;
            self.updooted = true;
        }
        Ok(())
    }

    fn downdoot(&self) -> Result<()> {
        reqwest::blocking::get(&format!("{}{}{}", HOSTNAME, "downvote/", self.id))?;
        Ok(())
    }
}

fn limit_str(data: &str, maxlength: usize) -> String {
    let graph = data.graphemes(true).collect::<Vec<&str>>();
    let slice = if graph.len() > maxlength {
        &graph[..maxlength]
    } else {
        &graph[..]
    };
    slice.join("")
}

fn build_song_widget() -> impl Widget<SongData> {
    let songlabel: Align<SongData> = Label::new(|data: &SongData, _env: &_| {
        format!(
            "{} - {} ",
            limit_str(&data.title, 40),
            limit_str(&data.artist, 30)
        )
    })
    .padding(5.0)
    .center();

    let playtimelabel: Align<SongData> =
        Label::new(|data: &SongData, _env: &_| format!("({}) <{}>", data.playtime, data.rating))
            .padding(5.0)
            .align_right();

    let skip = Button::new("✘")
        .on_click(|_: &mut EventCtx, song: &mut SongData, _: &Env| song.skip = true);

    let btn_upvote = Button::new(|song: &SongData, _: &Env| {
        if !song.updooted {
            "👍".to_string()
        } else {
            "👍✓".to_string()
        }
    })
    .on_click(|_: &mut EventCtx, song: &mut SongData, _: &Env| song.updoot().unwrap_or_default());

    let btn_downvote =
        Button::new("👎").on_click(|_: &mut EventCtx, song: &mut SongData, _: &Env| {
            song.downdoot().unwrap_or_default()
        });

    Flex::row()
        .with_child(skip)
        .with_child(btn_upvote)
        .with_child(btn_downvote)
        .with_child(songlabel)
        .with_flex_child(Align::right(playtimelabel), 1.0)
}
