use std::error;

use rodio::{source::Source, Decoder, OutputStream, OutputStreamHandle, Sink};
pub struct Player {
    stream_addr: Option<String>,
    stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Sink,
}

impl Player {
    pub fn new() -> Result<Self, Box<dyn error::Error>> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        Ok(Player {
            stream_addr: None,
            stream,
            stream_handle,
            sink,
        })
    }
}
