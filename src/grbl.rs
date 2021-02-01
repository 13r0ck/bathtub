extern crate serial;
use std::{thread, str};
use std::time::Duration;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver, SendError, TryRecvError};
use serial::prelude::*;
use serial::SystemPort;
use std::io::Write;

use chrono::prelude::*;
use std::io::BufReader;
use std::io::BufRead;


// used to clean up code when this file is imporded into another
#[derive(Debug)]
pub struct Grbl {
    pub sender: Sender<String>,
    pub receiver: Receiver<(chrono::DateTime<chrono::Local>, String, String)>,
}

impl Grbl {
    pub fn send(&self, gcode: String) -> Result<(), SendError<String>> {
        self.sender.send(gcode)
    }
    pub fn try_recv(&self) -> Result<(chrono::DateTime<chrono::Local>, String, String), TryRecvError> {
        self.receiver.try_recv()
    }
}

// Create new thread that, locks usb serial connection + used to send+recv gcode
pub fn new() -> Grbl {
    let (cnc_tx, ui_rx) = mpsc::channel();
    let (ui_tx, cnc_rx) = mpsc::channel();
    thread::spawn(move || {
        let mut port = get_port();
        let mut grbl_response: (DateTime<Local>, String, String);
        loop {
            if let Ok(command) = ui_rx.try_recv() {
                grbl_response = send(&mut port, command);
                ui_tx.send(grbl_response).unwrap();
            }
            thread::sleep(Duration::from_millis(1));
        }
    });
    Grbl {
        sender: cnc_tx,
        receiver: cnc_rx
    }
}

// used by new() to get the usb serial connection
fn get_port() -> SystemPort {
        let mut try_port = serial::open("/dev/ttyUSB0");
        if try_port.is_err() {
            let mut i = 1;
            while try_port.is_err() && i < 10000{
                try_port = serial::open(&format!("/dev/ttyUSB{}",i));
                i+=1;
            }
            if i == 10000 {
                panic!("unable to find USB port");
            }
        }
        let mut port = try_port.expect("port error");
        // default port settings for grbl, maybe should be configurable?
        port.reconfigure(&|settings| {
            settings.set_baud_rate(serial::Baud115200).unwrap();
            settings.set_char_size(serial::Bits8);
            settings.set_parity(serial::ParityNone);
            settings.set_stop_bits(serial::Stop1);
            settings.set_flow_control(serial::FlowNone);

            Ok(())
        }).unwrap();
        port.set_timeout(Duration::from_secs(60)).unwrap();
        port
}

// used by the new() thread to send to grbl and parse response
pub fn send(port: &mut SystemPort, gcode: String) -> (DateTime<Local>, String, String) {
    let mut buf = format!("{}\n", gcode.clone()).as_bytes().to_owned();
    port.write(&buf[..]).unwrap();
    let mut reader = BufReader::new(port);
    let mut line: String;
    let mut output: Vec<String> = Vec::new();
    buf = Vec::new();
    loop {
        // read until caridge return kek from grbl
        reader.read_until(0xD, &mut buf).unwrap();
        line = str::from_utf8(&buf).unwrap().to_string();
        // the first reponse from grbl initializing the connection is a bit weird, it has multiple
        // caridge returns, lockily it is the only one with a unicode 'null' char. GRBL doesnt do
        // anything with this first command, so we can mostly ignore it.
        if  line.contains("\u{0}\r") {
            return (Local::now(), gcode, "init".to_string());
        }
        if  line.contains("\r") {
            output.push(line.replace("\r",""));
            return (Local::now(), gcode.replace("\n","\\n"), output.into_iter().fold(String::new(), |mut string, part| {string.push_str(&part[..]); string}));
        }
        output.push(line);
    }
}
