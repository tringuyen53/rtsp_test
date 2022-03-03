//GSTREAMER

// This example demonstrates how to get a raw video frame at a given position
// and then rescale and store it with the image crate:

// {uridecodebin} - {videoconvert} - {appsink}

// The appsink enforces RGBx so that the image crate can use it. The sample layout is passed
// with the correct stride from GStreamer to the image crate as GStreamer does not necessarily
// produce tightly packed pixels, and in case of RGBx never.
extern crate gstreamer as gst;
extern crate gstreamer_app as gst_app;
extern crate gstreamer_video as gst_video;
use anyhow::Error;
use bastion::distributor::*;
use bastion::prelude::*;
use byte_slice_cast::*;
use derive_more::{Display, Error};
use gst::element_error;
use gst::glib;
use gst::prelude::*;
use gst::prelude::*;
use image::{DynamicImage, ImageFormat};
use rand::Rng;
use std::fs;
use std::io::{Cursor, Write}; // bring trait into scope
use tokio::runtime::Handle;
mod throttle;
use async_std::task;
use bastion::prelude::*;
use bytes::Bytes;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use job_scheduler::{Job, JobScheduler};
use nats::{self, asynk::Connection};
use rtp::packet::Packet;
use std::error::Error as OtherError;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use throttle::Throttle;
use tokio::net::TcpStream;
use webrtc_media::io::h264_writer::H264Writer;
use webrtc_media::io::Writer;
use webrtc_util::Unmarshal;
// #[path = "../examples-common.rs"]
// mod examples_common;

#[derive(Debug, Display, Error)]
#[display(fmt = "Missing element {}", _0)]
struct MissingElement(#[error(not(source))] &'static str);

#[derive(Debug, Display, Error)]
#[display(fmt = "Received error from {}: {} (debug: {:?})", src, error, debug)]
struct ErrorMessage {
    src: String,
    error: String,
    debug: Option<String>,
    source: glib::Error,
}

const NALU_TTYPE_STAP_A: u32 = 24;
const NALU_TTYPE_SPS: u32 = 7;
const NALU_TYPE_BITMASK: u32 = 0x1F;

fn is_key_frame(data: &[u8]) -> bool {
    if data.len() < 4 {
        false
    } else {
        let word = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let nalu_type = (word >> 24) & NALU_TYPE_BITMASK;
        (nalu_type == NALU_TTYPE_STAP_A && (word & NALU_TYPE_BITMASK) == NALU_TTYPE_SPS)
            || (nalu_type == NALU_TTYPE_SPS)
    }
}

const TOPIC: &str = "rtsp_test";
const URL: &str = "rtsp://10.50.13.252/1/h264major";

async fn connect_nats() -> Connection {
    nats::asynk::connect("nats://demo.nats.io:4222")
        .await
        .unwrap()
}

fn create_pipeline(uri: String, seed: u8) -> Result<gst::Pipeline, Error> {
    let client = task::block_on(connect_nats());
    gst::init()?;

    // Create our pipeline from a pipeline description string.
    // let pipeline = gst::parse_launch(&format!(
    //     "rtspsrc location={} latency=0 ! queue ! rtpjitterbuffer ! rtph264depay ! queue ! h264parse ! vaapih264dec ! queue ! videorate ! videoconvert ! videoscale ! jpegenc !  appsink name=sink ",
    //     uri
    // ))?
    //let pipeline = gst::parse_launch(&format!(
    //    "rtspsrc location={} latency=0 ! queue ! rtpjitterbuffer ! rtph264depay ! queue ! h264parse ! avdec_h264 ! queue ! video/x-raw ! jpegenc ! image/jpeg ! appsink name=sink" ,
    //      uri
    //    ))?
    let pipeline = gst::parse_launch(&format!(
        "rtspsrc location={} ! queue ! \
            video/x-h264,width=640,height=480,framerate=30/1,profile=baseline ! \
            h264parse ! \
            rtph264pay config-interval=2 pt=126 ! udpsink host=janus.3exp8.network port=5004",
        uri
    ))?
    // let pipeline = gst::parse_launch(&format!(
    //     "rtspsrc location={} latency=100 ! queue ! rtpjitterbuffer ! rtph264depay ! queue ! h264parse ! vaapih263dec ! queue ! videoconvert ! videoscale ! jpegenc ! appsink name=sink" ,
    //     uri
    // ))?
    .downcast::<gst::Pipeline>()
    .expect("Expected a gst::Pipeline");

    // blocking!(scheduler(count_2.clone()));
    // Getting data out of the appsink is done by setting callbacks on it.
    // The appsink will then call those handlers, as soon as data is available.

    pipeline.set_state(gst::State::Null)?;

    Ok(pipeline)
}
#[tokio::main]
async fn main() {
    // let handle = Handle::current();

    let urls = [
        // "rtsp://vietnam:L3xRay123!@10.50.30.212/1/h264major",
        // "rtsp://10.50.29.36/1/h264major",
        // "rtsp://10.50.31.171/1/h264major",
        // "rtsp://vietnam:L3xRay123!@10.50.12.187/media/video1",
        // "rtsp://vietnam:L3xRay123!@10.50.30.212/1/h264major",
        // "rtsp://10.50.31.171/1/h264major",
        //        "rtsp://10.50.29.36/1/h264major",
        //"rtsp://vietnam:L3xRay123!@10.50.12.187/media/video1",
        //        "rtsp://10.50.13.237/1/h264major",
        "rtsp://10.50.13.229/1/h264major",
        "rtsp://10.50.13.230/1/h264major",
        "rtsp://10.50.13.231/1/h264major",
        "rtsp://10.50.13.232/1/h264major",
        "rtsp://10.50.13.233/1/h264major",
        "rtsp://10.50.13.234/1/h264major",
        "rtsp://10.50.13.235/1/h264major",
        "rtsp://10.50.13.236/1/h264major",
        "rtsp://10.50.13.237/1/h264major",
        "rtsp://10.50.13.238/1/h264major",
        "rtsp://10.50.13.239/1/h264major",
        "rtsp://10.50.13.240/1/h264major",
        "rtsp://10.50.13.241/1/h264major",
        //     "rtsp://10.50.13.242/1/h264major",
        "rtsp://10.50.13.242/1/h264major",
        "rtsp://10.50.13.243/1/h264major",
        "rtsp://10.50.13.244/1/h264major",
        "rtsp://10.50.13.245/1/h264major",
        "rtsp://10.50.13.246/1/h264major",
        "rtsp://10.50.13.247/1/h264major",
        "rtsp://10.50.13.248/1/h264major",
        "rtsp://10.50.13.249/1/h264major",
        "rtsp://10.50.13.250/1/h264major",
        "rtsp://10.50.13.251/1/h264major",
        "rtsp://10.50.13.252/1/h264major",
        "rtsp://10.50.13.253/1/h264major",
        "rtsp://10.50.13.254/1/h264major",
        //"rtsp://10.50.31.171/1/h264major",
        //"rtsp://10.50.31.236/1/h264major",
        //"rtsp://10.50.14.39/1/h264major",
        //"rtsp://10.50.30.118/1/h264major",
        //"rtsp://10.50.31.241/axis-media/media.amp",
    ];

    Bastion::init();
    Bastion::supervisor(|supervisor| {
        supervisor.children(|children| {
            // Iniit staff
            // Staff (5 members) - Going to organize the event
            children
                .with_distributor(Distributor::named("rtsp"))
                .with_exec(get_rtsp_stream)
        })
    })
    .map_err(|_| println!("Error"));
    Bastion::supervisor(|supervisor| {
        supervisor.children(|children| {
            // Iniit staff
            // Staff (5 members) - Going to organize the event
            children
                .with_distributor(Distributor::named("rtsp-2"))
                .with_exec(get_rtsp_stream)
        })
    })
    .map_err(|_| println!("Error"));
    std::thread::sleep(std::time::Duration::from_secs(1));
    Bastion::supervisor(|supervisor| {
        supervisor.children(|children| {
            // Iniit staff
            // Staff (5 members) - Going to organize the event
            children
                .with_distributor(Distributor::named("transcode"))
                .with_exec(transcode_handler)
        })
    })
    .map_err(|_| println!("Error"));

    Bastion::start();
    std::thread::sleep(std::time::Duration::from_secs(2));
    let rtsp_actor = Distributor::named("rtsp");
    let rtsp2_actor = Distributor::named("rtsp-2");
    //    for url in urls {
    //        rtsp_actor.tell_one(url).expect("tell failed");
    //    }
    rtsp_actor
        .tell_one("rtsp://10.50.13.252/1/h264major")
        .expect("tell failed");
    //rtsp2_actor.tell_one("rtsp://10.50.13.254/1/h264major").expect("tell failed");
    //rtsp_actor.tell_one("rtsp://vietnam:L3xRay123!@10.50.12.187/media/video1").expect("tell failed");
    //println!("Result: {:?}", res);
    Bastion::block_until_stopped();
}

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn OtherError>> {
//     let mut stream = TcpStream::connect("rtsp://10.50.13.252:554/1/h264major").await?;
//     let mut data = Vec::new();
//     let n = stream.peek(&mut data).await?;
//     //let mut samples = data.to_vec();
//     println!("Lenght: {}", n);
//     println!("Samples: {:?}", data);
// 	Ok(())
// }

async fn get_rtsp_stream(ctx: BastionContext) -> Result<(), ()> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    //let mut rng = rand::thread_rng();
    loop {
        MessageHandler::new(ctx.recv().await?)
            .on_tell(|message: &str, _| {
                //let mut rng = rand::thread_rng();
                //let n1: u8 = rng.gen();
                //println!("spawn new actor: {:?} - {:?}", message, n1);
                rt.spawn_blocking(move || {
                    create_pipeline(message.to_owned(), 1).and_then(|pipeline| main_loop(pipeline));
                    //let pipeline = create_pipeline(message.to_owned(), n1).await.unwrap();
                    //                  main_loop(pipeline)
                });
            })
            .on_fallback(|unknown, _sender_addr| {
                println!("unknown");
            });
    }
}

async fn transcode_handler(ctx: BastionContext) -> Result<(), ()> {
    loop {
        MessageHandler::new(ctx.recv().await?)
            .on_tell(|message: Vec<u8>, _| {
                println!("Receive frame from rtsp: {}", message.len());
            })
            .on_fallback(|unknown, _sender_addr| {
                println!("unknown");
            });
    }
}

fn scheduler(count: Arc<Mutex<i32>>) {
    let mut sched = JobScheduler::new();

    sched.add(Job::new("0/1 * * * * *".parse().unwrap(), || {
        // check_time_config(recording_scheduler);
        *count.lock().unwrap() = 0;

        println!("I get executed every 23:59!");
    }));

    loop {
        sched.tick();

        std::thread::sleep(Duration::from_secs(30));
    }
}

fn get_bit_by_pos(buffer: &[u8], pos: usize) -> i32 {
    (buffer[pos / 8] >> (8 - pos % 8) & 0x01).into()
}

fn decode_golomb(byte_stream: &[u8], index: usize) -> (i32, usize) {
    let mut leading_zero_bits = -1;
    let mut code_num = 0;
    let mut pos = index;

    if byte_stream.len() == 0 || pos == 0 {
        (0, index)
    } else {
        let mut b = 0;
        while b != 0 {
            b = get_bit_by_pos(byte_stream, pos);
            pos = pos + 1;

            leading_zero_bits = leading_zero_bits + 1;
        }

        let mut c = leading_zero_bits;

        while c > 0 {
            code_num = code_num | (get_bit_by_pos(byte_stream, pos) << (c - 1));
            pos = pos + 1;

            c = c - 1;
        }

        ((1 << leading_zero_bits) - 1 + code_num, pos)
    }
}
