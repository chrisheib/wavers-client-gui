#![windows_subsystem = "windows"]

use serde_derive::{Deserialize, Serialize};

//"C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64\rc.exe"
//"C:\Strawberry\c\x86_64-w64-mingw32\bin\ar.exe"
//"C:\Strawberry\c\bin\windres.exe"

use druid::{
    widget::{Align, Button, Flex, Label, List, Slider, ViewSwitcher},
    BoxConstraints, Color, Env, Event, EventCtx, FontDescriptor, FontFamily, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, Size, TimerToken, UpdateCtx,
};
use druid::{AppLauncher, Data, Lens, Widget, WidgetExt, WindowDesc};
use stable_eyre::Result;
use std::time::Duration;
use std::{sync::Arc, time::Instant};
use unicode_segmentation::UnicodeSegmentation;

const TIMER_INTERVAL: Duration = Duration::from_millis(100);
const WINDOW_WIDTH: f64 = 750f64;
const WINDOW_HEIGHT: f64 = 780f64;

#[derive(Clone, Serialize, Deserialize)]
struct MyConfig {
    default_volume: f64,
    hostname: String,
    port: String,
}

impl ::std::default::Default for MyConfig {
    fn default() -> Self {
        Self {
            default_volume: 0.30f64,
            hostname: "localhost".into(),
            port: "81".into(),
        }
    }
}

struct TimerWidget {
    timer_id: TimerToken,
    fps_timer_id: TimerToken,
}

impl Widget<DruidState> for TimerWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut DruidState, _env: &Env) {
        match event {
            Event::WindowConnected => {
                // Start the timer when the application launches
                self.timer_id = ctx.request_timer(TIMER_INTERVAL);
                self.fps_timer_id = ctx.request_timer(Duration::from_millis(10));
                // Start first Song
                data.dl_play().unwrap_or(());
            }
            Event::Timer(id) => {
                if *id == self.timer_id {
                    self.timer_id = ctx.request_timer(TIMER_INTERVAL);
                    timer_tick(ctx, data);
                }
                if *id == self.fps_timer_id {
                    self.fps_timer_id = ctx.request_timer(Duration::from_millis(10));
                    ctx.window().invalidate();
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
                data.queue_song(SongData::fetch_random_song(data).unwrap())
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
    #[data(ignore)]
    config: MyConfig,
    last_song: SongData,
}

fn main() -> Result<()> {
    println!("vor window");
    let main_window = WindowDesc::new(ui_builder)
        .with_min_size((WINDOW_WIDTH, WINDOW_HEIGHT))
        .window_size((WINDOW_WIDTH, WINDOW_HEIGHT))
        .title("Wavers");

    println!("nach window vor config");

    let cfg: MyConfig = confy::load_path("wavers-gui.conf")?;

    println!("nach config vor audio");

    let (stream, handle) = rodio::OutputStream::try_default()?;

    println!("nach audio vor state");

    let mut state = DruidState {
        handle: Arc::new(handle),
        sink: None,
        volume: cfg.default_volume,
        last_timestamp: Instant::now(),
        playtime: 0,
        items: Default::default(),
        current_song: SongData::default(),
        paused: false,
        config: cfg,
        last_song: SongData::default(),
    };

    println!("nach state vor fetch");

    for _ in 0..7 {
        state.queue_song(SongData::fetch_random_song(&state)?);
    }

    println!("2");

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
        fps_timer_id: TimerToken::INVALID,
    };

    let songnamelabel: Align<DruidState> =
        Label::new(|data: &DruidState, _: &_| limit_str(&data.current_song.title, 80)).center();
    let albumlabel: Align<DruidState> =
        Label::new(|data: &DruidState, _: &_| limit_str(&data.current_song.album, 80)).center();

    let id = Label::new(|data: &DruidState, _env: &_| data.current_song.id.to_string())
        .with_text_color(Color::grey(0.60))
        .with_text_size(10f64);

    let rating_full =
        Label::new(|data: &DruidState, _env: &_| rating_to_fullstars(data.current_song.rating))
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(18f64)
            .align_right()
            .padding(-2.0);

    let rating_empty =
        Label::new(|data: &DruidState, _env: &_| rating_to_emptystars(data.current_song.rating))
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(18f64)
            .with_text_color(Color::grey(0.50))
            .align_left()
            .padding(-2.0);

    let rating = Flex::row().with_child(rating_full).with_child(rating_empty);

    let artistlabel: Align<DruidState> =
        Label::new(|data: &DruidState, _: &_| limit_str(&data.current_song.artist, 80)).center();

    let progresslabel: Align<DruidState> = Label::new(|data: &DruidState, _env: &_| {
        format!(
            "{} / {}",
            format_songlength((data.playtime / 1000) as u64),
            data.current_song.playtime
        )
    })
    .padding(5.0)
    .center();

    let volume_small: Label<DruidState> = Label::new("🔉".to_string());
    let volume_big: Label<DruidState> = Label::new("🔊".to_string());

    let volumeslider = Slider::new().lens(DruidState::volume);

    let volumerow = Flex::row()
        .with_child(volume_small)
        .with_child(volumeslider)
        .with_child(volume_big);

    let songqueue = List::new(build_song_widget).lens(DruidState::items);

    let buttonrow = Flex::row()
        .with_child(btn_pauseplay)
        .with_child(btn_skip)
        .with_child(btn_upvote)
        .with_child(btn_downvote);

    let songpanelinner = Flex::column()
        .with_spacer(4.0)
        .with_child(songnamelabel)
        .with_child(albumlabel)
        .with_child(artistlabel)
        .with_child(rating)
        .with_spacer(2.0)
        .with_child(id)
        .with_spacer(4.0);

    let songpanelouter = Flex::row()
        .with_spacer(4.0)
        .with_child(songpanelinner)
        .with_spacer(4.0)
        .border(Color::grey(0.07), 1.0)
        .rounded(3.0);

    let songrow = Flex::row()
        .with_flex_spacer(1.0)
        .with_child(songpanelouter)
        .with_flex_spacer(1.0);

    let last_song_panel = ViewSwitcher::new(
        |data: &DruidState, _env| data.last_song.real_song,
        |selector, _data, _env| match selector {
            true => Box::new(build_song_widget().lens(DruidState::last_song)),
            false => Box::new(Flex::row()),
        },
    );

    let body = Flex::column()
        .with_child(last_song_panel)
        .with_child(buttonrow)
        .with_child(timer1)
        .with_spacer(5.0)
        .with_child(songrow)
        .with_spacer(5.0)
        .with_child(progresslabel)
        .with_child(volumerow)
        .with_spacer(5.0)
        .with_flex_child(songqueue, 1.0);

    Flex::row()
        .with_spacer(3.0)
        .with_flex_child(body, 1.0)
        .with_spacer(3.0)
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
        Ok(reqwest::blocking::get(&format!(
            "http://{}:{}/songs/{}",
            self.config.hostname, self.config.port, id
        ))?
        .bytes()?
        .to_vec())
    }

    fn dl_play(&mut self) -> Result<()> {
        let a = self.drop_song(0)?;
        self.last_song = std::mem::replace(&mut self.current_song, a);
        self.last_song.is_last = true;
        let song = self.dl()?;
        self.play(song)?;
        self.queue_song(SongData::fetch_random_song(&self)?);
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
            Err(stable_eyre::eyre::eyre!("invalid song index!"))
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
    #[data(ignore)]
    config: MyConfig,
    real_song: bool,
    is_last: bool,
    downdooted: bool,
}

impl SongData {
    fn fetch_random_song(data: &DruidState) -> Result<SongData> {
        let id = reqwest::blocking::get(&format!(
            "http://{}:{}/random_id",
            data.config.hostname, data.config.port
        ))?
        .text()?;

        let mut result = SongData {
            config: data.config.clone(),
            id,
            real_song: true,
            ..Default::default()
        };

        result.fetch_songdata()?;

        Ok(result)
    }

    fn fetch_songdata(&mut self) -> Result<()> {
        if !self.real_song {
            return Ok(());
        }
        let songdata = reqwest::blocking::get(&format!(
            "http://{}:{}/songdata/{}",
            self.config.hostname, self.config.port, self.id
        ))?
        .text()?;
        let songdata = json::parse(&songdata)?;

        let mut title = songdata["songname"].to_string();
        if title.is_empty() {
            title = songdata["filename"].to_string()
        };

        self.title = title;
        self.artist = songdata["artist"].to_string();
        self.album = songdata["album"].to_string();
        self.playtime = songdata["length"].to_string();
        self.rating = songdata["rating"].to_string().parse().unwrap_or_default();
        self.skip = false;

        Ok(())
    }

    fn updoot(&mut self) -> Result<()> {
        if !self.real_song {
            return Ok(());
        }
        if self.downdooted {
            // downdooted -> updooted
            self.net_updoot()?;
            self.net_updoot()?;
            self.downdooted = false;
            self.updooted = true;
        } else if !self.updooted {
            // neutral -> updooted
            self.net_updoot()?;
            self.updooted = true;
        } else {
            // updooted -> neutral
            self.net_downdoot()?;
            self.updooted = false;
        }
        self.fetch_songdata()?;
        Ok(())
    }

    fn downdoot(&mut self) -> Result<()> {
        if !self.real_song {
            return Ok(());
        }
        if self.updooted {
            // updooted -> downdooted
            self.net_downdoot()?;
            self.net_downdoot()?;
            self.updooted = false;
            self.downdooted = true;
        } else if !self.downdooted {
            // neutral -> downdooted
            self.net_downdoot()?;
            self.downdooted = true;
        } else {
            // downdooted -> neutral
            self.net_updoot()?;
            self.downdooted = false;
        }
        self.fetch_songdata()?;
        Ok(())
    }

    fn net_updoot(&mut self) -> Result<()> {
        reqwest::blocking::get(&format!(
            "http://{}:{}/upvote/{}",
            self.config.hostname, self.config.port, self.id
        ))?;
        Ok(())
    }
    fn net_downdoot(&mut self) -> Result<()> {
        reqwest::blocking::get(&format!(
            "http://{}:{}/downvote/{}",
            self.config.hostname, self.config.port, self.id
        ))?;
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

fn rating_to_fullstars(rating: u32) -> String {
    (0..rating).map(|_| "★").collect::<String>()
}

fn rating_to_emptystars(rating: u32) -> String {
    (rating..7).map(|_| "☆").collect::<String>()
}

fn build_song_widget() -> impl Widget<SongData> {
    let songlabelname: Align<SongData> =
        Label::new(|data: &SongData, _env: &_| limit_str(&data.title, 80))
            .padding(1.0)
            .align_left();

    let songlabelalbum: Align<SongData> =
        Label::new(|data: &SongData, _env: &_| limit_str(&data.album, 80))
            .padding(1.0)
            .align_left();

    let songlabelartist: Align<SongData> =
        Label::new(|data: &SongData, _env: &_| limit_str(&data.artist, 80))
            .padding(1.0)
            .align_left();

    let playtimelabel: Align<SongData> =
        Label::new(|data: &SongData, _env: &_| format!("({})", data.playtime))
            .padding(5.0)
            .align_right();

    let id = Label::new(|data: &SongData, _env: &_| data.id.to_string())
        .with_text_color(Color::grey(0.50))
        .with_text_size(10f64);

    let rating_full = Label::new(|data: &SongData, _env: &_| rating_to_fullstars(data.rating))
        .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
        .with_text_size(18f64)
        .align_right()
        .padding(-2.0);

    let rating_empty = Label::new(|data: &SongData, _env: &_| rating_to_emptystars(data.rating))
        .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
        .with_text_size(18f64)
        .with_text_color(Color::grey(0.50))
        .align_left()
        .padding(-2.0);

    let rating_row = Flex::row().with_child(rating_full).with_child(rating_empty);

    let skip_panel = ViewSwitcher::new(
        |data: &SongData, _env| data.is_last,
        |selector, _data, _env| match selector {
            false => Box::new(
                Button::new("✘")
                    .on_click(|_: &mut EventCtx, song: &mut SongData, _: &Env| song.skip = true),
            ),
            true => Box::new(Flex::column()),
        },
    );

    let btn_upvote = Button::new(|song: &SongData, _: &Env| {
        if !song.updooted {
            "👍".to_string()
        } else {
            "👍✓".to_string()
        }
    })
    .on_click(|_: &mut EventCtx, song: &mut SongData, _: &Env| song.updoot().unwrap_or_default());

    let btn_downvote = Button::new(|song: &SongData, _: &Env| {
        if !song.downdooted {
            "👎".to_string()
        } else {
            "👎✓".to_string()
        }
    })
    .on_click(|_: &mut EventCtx, song: &mut SongData, _: &Env| {
        song.downdoot().unwrap_or_default();
        song.skip = true;
    });

    let names = Flex::column()
        .with_spacer(3.0)
        .with_child(songlabelname)
        .with_child(songlabelalbum)
        .with_child(songlabelartist)
        .with_spacer(3.0);

    let controls = Flex::row()
        .with_child(skip_panel)
        .with_child(btn_upvote)
        .with_child(btn_downvote);

    let left = Flex::column()
        .with_child(rating_row)
        .with_spacer(3f64)
        .with_child(controls)
        .with_spacer(3f64)
        .with_child(id);

    Flex::row()
        .with_spacer(3.0)
        .with_child(left)
        .with_spacer(5.0)
        .with_flex_child(Align::left(names), 1.0)
        .with_child(Align::right(playtimelabel))
        .border(Color::grey(0.07), 1.0)
        .rounded(3.0)
        .padding(1.0)
}
