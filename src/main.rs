use anyhow::Result;
use druid::widget::{
    Button,
    Flex, //, Label
};
use druid::{
    AppLauncher,
    Data,
    Lens, //, LocalizedString
    Widget,
    WidgetExt,
    WindowDesc,
};
use rodio::OutputStreamHandle;
use std::io::BufReader;
use std::{fs::File, sync::Arc};

const HOSTNAME: &str = "http://localhost:81/";
const LOCAL_FILENAME: &str = "song.mp3";

//fn main() -> iced::Result {
// fn oldmain() -> Result<()> {
//     //Counter::run(Settings::default())
//     //loop {}

//     let (_stream, handle) = rodio::OutputStream::try_default()?;
//     let sink = rodio::Sink::try_new(&handle)?;
//     //let (handle, sink) = get_sink()?;
//     dl_play(&sink)?;
//     // dl()?;
//     // oldplay()?;
//     Ok(())
// }

#[derive(Clone, Data, Lens)]
struct DruidState {
    handle: Arc<rodio::OutputStreamHandle>,
    sink: Option<Arc<rodio::Sink>>,
}

fn main() -> Result<()> {
    let main_window = WindowDesc::new(ui_builder);

    let (stream, handle) = rodio::OutputStream::try_default()?;

    let state = DruidState {
        handle: Arc::new(handle),
        sink: None,
    };

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(state)?;

    // Stream darf erst hier gedroppt werden, sonst zeigt das Handle ins nichts -> Sound stoppt.
    drop(stream);

    Ok(())
}

fn ui_builder() -> impl Widget<DruidState> {
    // The label text will be computed dynamically based on the current locale and count
    //let text =
    //    LocalizedString::new("hello-counter").with_arg("count", |data: DruidState, _env| data);
    //let label = Label::new(text).padding(5.0).center();
    let button = Button::new("play/skip")
        .on_click(|_ctx, data: &mut DruidState, _env| {
            dl_play(&mut data.sink, &data.handle).unwrap_or(())
        })
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

    Flex::column()
        //.with_child(label)
        .with_child(button)
        .with_child(button2)
}

fn dl_play(sink: &mut Option<Arc<rodio::Sink>>, handle: &OutputStreamHandle) -> Result<()> {
    dl()?;
    play(sink, handle)?;
    Ok(())
}

fn dl() -> Result<()> {
    std::fs::remove_file(LOCAL_FILENAME).unwrap_or(());
    let id = reqwest::blocking::get(&format!("{}{}", HOSTNAME, "random_id"))?.text()?;
    let response = reqwest::blocking::get(&format!("{}{}{}", HOSTNAME, "songs/", id))?.bytes()?;
    let mut file = File::create(LOCAL_FILENAME)?;
    std::io::copy(&mut response.as_ref(), &mut file)?;
    Ok(())
}

fn play(sinkopt: &mut Option<Arc<rodio::Sink>>, handle: &OutputStreamHandle) -> Result<()> {
    let file = File::open(LOCAL_FILENAME)?;
    let sink;
    if let Some(s) = sinkopt {
        s.stop();
        *sinkopt = None;
    }

    *sinkopt = Some(Arc::new(rodio::Sink::try_new(handle)?));

    // Unwrap als Assert. Es muss eine neue Sink geben.
    sink = sinkopt.as_ref().unwrap();
    if !sink.empty() {
        sink.stop();
    }
    sink.set_volume(0.05);
    sink.append(rodio::Decoder::new(BufReader::new(file))?);
    Ok(())
}

// fn oldplay() -> Result<()> {
//     let (_stream, handle) = rodio::OutputStream::try_default()?;
//     let sink = rodio::Sink::try_new(&handle)?;
//     let file = File::open(LOCAL_FILENAME)?;
//     sink.append(rodio::Decoder::new(BufReader::new(file))?);
//     sink.sleep_until_end();
//     Ok(())
// }

// fn get_sink() -> Result<(rodio::OutputStreamHandle, rodio::Sink)> {
//     let (_stream, handle) = rodio::OutputStream::try_default()?;
//     let sink = rodio::Sink::try_new(&handle)?;
//     Ok((handle, sink))
// }
