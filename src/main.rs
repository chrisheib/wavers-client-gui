use anyhow::Result;
use iced::{button, Align, Button, Column, Element, Sandbox, Settings, Text};
use rodio::Source;
use std::fs::File;
use std::io::BufReader;
//use tokio::runtime::Runtime;

const HOSTNAME: &str = "http://localhost:81/songs/random";
const LOCAL_FILENAME: &str = "song.mp3";

//fn main() -> iced::Result {
fn main() -> Result<()> {
    //Counter::run(Settings::default())
    dl()?;
    play()?;
    //loop {}
    Ok(())
}

//#[derive(Default)]
struct Counter {
    value: i32,
    dl_button: button::State,
    //streamhandle: rodio::OutputStreamHandle,
    //rt: tokio::runtime::Runtime,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    DownloadFile,
}

//  todo: Application!
impl Sandbox for Counter {
    type Message = Message;

    fn new() -> Self {
        Self {
            value: i32::default(),
            dl_button: button::State::default(),
        }
    }

    fn title(&self) -> String {
        String::from("Counter - Iced")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::DownloadFile => {
                dl().unwrap();
                play().unwrap();
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        Column::new()
            .padding(20)
            .align_items(Align::Center)
            .push(Text::new(self.value.to_string()).size(50))
            .push(
                Button::new(&mut self.dl_button, Text::new("Download Random"))
                    .on_press(Message::DownloadFile),
            )
            .into()
    }
}

fn dl() -> Result<()> {
    //async fn dl() {
    //let client = Client::new();
    //let mut response = client.get(GL_hostname).send().await?;
    std::fs::remove_file(LOCAL_FILENAME).unwrap_or(());

    let response = reqwest::blocking::get(HOSTNAME)?.bytes()?;
    let mut file = File::create(LOCAL_FILENAME)?;
    std::io::copy(&mut response.as_ref(), &mut file)?;
    Ok(())
}

fn play() -> Result<()> {
    let (_stream, handle) = rodio::OutputStream::try_default()?;
    let file = File::open(LOCAL_FILENAME)?;
    let sink = rodio::Sink::try_new(&handle)?;
    sink.append(rodio::Decoder::new(BufReader::new(file))?);
    sink.sleep_until_end();

    Ok(())
}
