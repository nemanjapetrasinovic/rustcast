#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::{
    io::{Read, Seek},
    thread,
    time::Duration,
};

mod player;
use eframe::egui;
use log::error;
use rodio::{source::Source, Decoder, OutputStream, OutputStreamHandle, Sink};

fn main() {
    env_logger::init();

    thread::spawn(|| {
        let r = FifoRead::from_str("https://stream.daskoimladja.com:9000/stream").unwrap();
        let source = Decoder::new(r).unwrap();
        println!("{:?}", &source.channels());
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        // let source = rodio::source::SineWave::new(1000.0).take_duration(sound_duration);
        sink.append(source);
        sink.sleep_until_end();
    });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        native_options,
        Box::new(|cc| Box::new(MyEguiApp::new(cc))),
    )
    .unwrap_or_else(|e| error!("An error occured {}", e));
    // sink.play();
    // sink.append(source);
    // sink.set_volume(1.0);
}

struct FifoRead {
    pub reader: Box<dyn Read + Send + Sync + 'static>,
}

impl FifoRead {
    fn from_str(addr: &str) -> Result<Self, ureq::Error> {
        let r = ureq::get(addr).call();
        match r {
            Ok(r) => Ok(FifoRead {
                reader: r.into_reader(),
            }),
            Err(e) => Err(e),
        }
    }

    // fn new() -> Self {
    //     FifoRead {
    //         reader: ureq::get("https://stream.daskoimladja.com:9000/stream").call().unwrap().into_reader()
    //     }
    // }
}

impl Read for FifoRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }
}

impl Seek for FifoRead {
    fn seek(&mut self, _pos: std::io::SeekFrom) -> std::io::Result<u64> {
        Ok(0)
    }
}

#[derive(Default)]
struct MyEguiApp {}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self::default()
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.add(egui::Button::new("Play")).clicked() {
                error!("Clicked");
            }
        });
    }
}
