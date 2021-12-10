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
use gst::element_error;
use gst::glib;
use gst::prelude::*;
use byte_slice_cast::*;
use std::io::Write; // bring trait into scope
use std::fs;
use tokio::runtime::Handle;
use rand::Rng;
use anyhow::Error;
use derive_more::{Display, Error};
use image::{DynamicImage, ImageFormat};

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

fn create_pipeline(uri: String, seed: u8) -> Result<gst::Pipeline, Error> {
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
         "rtspsrc location={} latency=0 ! queue ! rtpjitterbuffer ! rtph264depay ! queue ! h264parse ! vaapih264dec ! queue ! video/x-raw ! jpegenc ! image/jpeg ! appsink name=sink" ,
         uri
     ))?
    // let pipeline = gst::parse_launch(&format!(
    //     "rtspsrc location={} latency=0 ! queue ! rtpjitterbuffer ! rtph264depay ! queue ! h264parse ! vaapih263dec ! queue ! videoconvert ! videoscale ! jpegenc ! appsink name=sink" ,
    //     uri
    // ))?
    .downcast::<gst::Pipeline>()
    .expect("Expected a gst::Pipeline");

    println!("pipeline: {:?}", pipeline);
    // Get access to the appsink element.
    let appsink = pipeline
        .by_name("sink")
        .expect("Sink element not found")
        .downcast::<gst_app::AppSink>()
        .expect("Sink element is expected to be an appsink!");

    println!("appsink: {:?}", appsink);

    // Don't synchronize on the clock, we only want a snapshot asap.
    // appsink.set_property("sync", false);

    // Tell the appsink what format we want.
    // This can be set after linking the two objects, because format negotiation between
    // both elements will happen during pre-rolling of the pipeline.
    // appsink.set_caps(Some(
    //     &gst::Caps::builder("video/x-h264")
    //         // .field("framerate", gst::Fraction::new(5, 1))
    //         .field("stream-format", "byte-stream")
    //         .build(),
    // ));
    println!("Before callback");
    // let mut got_snapshot = false;

    let mut count = 0;
    // Getting data out of the appsink is done by setting callbacks on it.
    // The appsink will then call those handlers, as soon as data is available.
    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            // Add a handler to the "new-sample" signal.
            .new_sample(move |appsink| {
                // Pull the sample in question out of the appsink's buffer.
                let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                println!("Sample: {:?}", sample);
                let buffer = sample.buffer().ok_or_else(|| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to get buffer from appsink")
                    );

                    gst::FlowError::Error
                })?;

                println!("Buffer {:?}", buffer);
                

                let map = buffer.map_readable().map_err(|_| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to map buffer readable")
                    );

                    gst::FlowError::Error
                })?;
                println!("xxxxxxxx Map {:?}", map);   

                let samples = map.as_slice_of::<u8>().map_err(|_| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to interprete buffer as S16 PCM")
                    );

                    gst::FlowError::Error
                })?;

                 //SAVE IMAGE
                // let mut file = fs::File::create(format!("img-{}.jpg", count)).unwrap();
                // file.write_all(samples);

                let img_result = 
                    image::load_from_memory_with_format(samples, ImageFormat::Jpeg);
                match img_result {
                    Ok(image) => {
                            image.save(format!("img-{}.jpg", count)).unwrap();
                            // match res {
                            //     Ok(_) => count += 1,
                            //     Err(_) => count += 1
                            // }
                            count += 1;
                        },
                    Err(_) => (),
                };
                
                // let img16 = img.into_rgb8();
                // let data = img16.into_raw() as Vec<u8>;
                // println!("Image length: {}", data.len());
                
                Ok(gst::FlowSuccess::Ok)
                // Err(gst::FlowError::Error)
            })
            .build(),
    );

    Ok(pipeline)
}

fn main_loop(pipeline: gst::Pipeline) -> Result<(), Error> {
    println!("Start main loop");
    pipeline.set_state(gst::State::Playing)?;

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    println!("Bus: {:?}", bus);

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        // println!("In loop msg: {:?}", msg);
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null)?;
                return Err(ErrorMessage {
                    src: msg
                        .src()
                        .map(|s| String::from(s.path_string()))
                        .unwrap_or_else(|| String::from("None")),
                    error: err.error().to_string(),
                    debug: err.debug(),
                    source: err.error(),
                }
                .into());
            }
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;

    Ok(())
}
#[tokio::main]
async fn main() {
    let handle = Handle::current();

    let urls = [
        // "rtsp://vietnam:L3xRay123!@10.50.30.212/1/h264major",
        // "rtsp://10.50.29.36/1/h264major",
        // "rtsp://10.50.31.171/1/h264major",
        // "rtsp://vietnam:L3xRay123!@10.50.12.187/media/video1",
        "rtsp://10.50.13.237/1/h264major",
        "rtsp://10.50.13.233/1/h264major",
        "rtsp://10.50.13.234/1/h264major",
        "rtsp://10.50.13.235/1/h264major",
        "rtsp://10.50.13.236/1/h264major",
    ];
    let mut rng = rand::thread_rng();

    for url in urls {
        let n1: u8 = rng.gen();
        handle.spawn_blocking(move || {  
            match create_pipeline(url.to_owned(), n1).and_then(|pipeline| main_loop(pipeline)) {
                    Ok(r) => r,
                    Err(e) => println!("Error! {}", e),
                } 
            });
    }

    loop {}
}
// fn main() {
//     // use std::env;

//     // let mut args = env::args();

//     // Parse commandline arguments: input URI, position in seconds, output path
//     // let _arg0 = args.next().unwrap();
//     // let uri = args
//     //     .next()
//     //     .expect("No input URI provided on the commandline");
//     // let position = args
//     //     .next()
//     //     .expect("No position in second on the commandline");
//     // let position = position
//     //     .parse::<u64>()
//     //     .expect("Failed to parse position as integer");
//     // let out_path = args
//     //     .next()
//     //     .expect("No output path provided on the commandline");
//     // let uri = "rtsp://10.50.13.231/1/h264major".to_owned();
//     // let position = 10;
//     // let out_path = "/img";
//     // let out_path = std::path::PathBuf::from(out_path);

//     match create_pipeline(uri).and_then(|pipeline| main_loop(pipeline)) {
//         Ok(r) => r,
//         Err(e) => eprintln!("Error! {}", e),
//     }
// }
