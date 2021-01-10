use anyhow::Result;
use druid::{
    widget::{Button, Flex, Slider},
    Event,
};
use druid::{AppLauncher, Data, Lens, Widget, WidgetExt, WindowDesc};
use std::{fs::File, sync::Arc};
use std::{io::BufReader, time::Duration};

const HOSTNAME: &str = "http://localhost:81/";
const LOCAL_FILENAME: &str = "song.mp3";
const TIMER_INTERVAL: Duration = Duration::from_millis(100);
const DEFAULT_VOLUME: f64 = 0.45f64;

struct TimerWidget {
    timer_id: druid::TimerToken,
}

impl Widget<DruidState> for TimerWidget {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut DruidState,
        _env: &druid::Env,
    ) {
        match event {
            Event::WindowConnected => {
                // Start the timer when the application launches
                self.timer_id = ctx.request_timer(TIMER_INTERVAL);
                // Start first Song
                dl_play(data).unwrap_or(());
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
        _ctx: &mut druid::LifeCycleCtx,
        _event: &druid::LifeCycle,
        _data: &DruidState,
        _env: &druid::Env,
    ) {
    }

    fn update(
        &mut self,
        _ctx: &mut druid::UpdateCtx,
        _old_data: &DruidState,
        _data: &DruidState,
        _env: &druid::Env,
    ) {
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        _data: &DruidState,
        _env: &druid::Env,
    ) -> druid::Size {
        bc.constrain((0.0, 0.0))
    }

    fn paint(&mut self, _ctx: &mut druid::PaintCtx, _data: &DruidState, _env: &druid::Env) {}
}

fn timer_tick(_ctx: &mut druid::EventCtx, data: &mut DruidState) {
    //println!("tick :)");

    if let Some(sink) = data.sink.as_ref() {
        if sink.empty() {
            println!("NEUES LIED!");
            dl_play(data).unwrap_or(());
        } else {
            sink.set_volume(data.corrected_volume());
        }
    }
}

#[derive(Clone, Data, Lens)]
struct DruidState {
    handle: Arc<rodio::OutputStreamHandle>,
    sink: Option<Arc<rodio::Sink>>,
    volume: f64,
    songname: String,
    id: u32,
}

fn main() -> Result<()> {
    let main_window = WindowDesc::new(ui_builder).window_size((100f64, 50f64));

    let (stream, handle) = rodio::OutputStream::try_default()?;

    let state = DruidState {
        handle: Arc::new(handle),
        sink: None,
        volume: DEFAULT_VOLUME,
        songname: "None".to_string(),
        id: 0,
    };

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(state)?;

    // Stream darf erst hier gedroppt werden, sonst zeigt das Handle ins nichts -> Sound stoppt.
    drop(stream);

    Ok(())
}

fn ui_builder() -> impl Widget<DruidState> {
    let button = Button::new("play/skip")
        .on_click(|_ctx, data: &mut DruidState, _env| dl_play(data).unwrap_or(()))
        .padding(5.0);
    let button2 = Button::new("pause/play")
        .on_click(|_ctx, data: &mut DruidState, _env| {
            if let Some(ref sink) = data.sink {
                if sink.is_paused() {
                    sink.play()
                } else {
                    sink.pause()
                }
            }
        })
        .padding(5.0);

    let timer1 = TimerWidget {
        timer_id: druid::TimerToken::INVALID,
    };

    let songnamelabel: druid::widget::Align<DruidState> =
        druid::widget::Label::new(|data: &DruidState, _env: &_| {
            format!("Playing: {}", data.songname)
        })
        .padding(5.0)
        .center();

    let volumelabel: druid::widget::Align<DruidState> =
        druid::widget::Label::new(|data: &DruidState, _env: &_| {
            format!("Volume: {:.2}", data.volume)
        })
        .padding(5.0)
        .center();
    let volumeslider = Slider::new().lens(DruidState::volume);

    Flex::column()
        .with_child(button)
        .with_child(button2)
        .with_child(timer1)
        .with_child(songnamelabel)
        .with_child(volumelabel)
        .with_child(volumeslider)
}

fn dl_play(data: &mut DruidState) -> Result<()> {
    dl(data)?;
    set_songtitle(data)?;
    play(data)?;
    Ok(())
}

fn set_songtitle(data: &mut DruidState) -> Result<()> {
    let songdata =
        reqwest::blocking::get(&format!("{}{}{}", HOSTNAME, "songdata/", data.id))?.text()?;
    let songdata = json::parse(&songdata)?;
    let title = songdata["songname"].to_string();
    data.songname = if !title.is_empty() {
        title
    } else {
        songdata["filename"].to_string()
    };
    Ok(())
}

fn dl(data: &mut DruidState) -> Result<()> {
    std::fs::remove_file(LOCAL_FILENAME).unwrap_or(());
    let id = reqwest::blocking::get(&format!("{}{}", HOSTNAME, "random_id"))?.text()?;
    let response = reqwest::blocking::get(&format!("{}{}{}", HOSTNAME, "songs/", id))?.bytes()?;
    let mut file = File::create(LOCAL_FILENAME)?;
    std::io::copy(&mut response.as_ref(), &mut file)?;
    data.id = id.parse()?;
    Ok(())
}

/// Kill old sink and create a new one with the handle from the DruidState.
/// Play the local sound file.
fn play(data: &mut DruidState) -> Result<()> {
    let file = File::open(LOCAL_FILENAME)?;
    let sink;
    if let Some(s) = data.sink.as_ref() {
        s.stop();
        data.sink = None;
    }

    data.sink = Some(Arc::new(rodio::Sink::try_new(&data.handle)?));

    // Unwrap als Assert. Es muss eine neue Sink geben.
    sink = data.sink.as_ref().unwrap();
    if !sink.empty() {
        sink.stop();
    }
    sink.set_volume(data.corrected_volume());
    sink.append(rodio::Decoder::new(BufReader::new(file))?);
    Ok(())
}

impl DruidState {
    /// Corrects for dumb brain.
    fn corrected_volume(&self) -> f32 {
        (self.volume * self.volume * self.volume) as f32
    }
}
