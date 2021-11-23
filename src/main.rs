// use opencv::{Result, imgcodecs, prelude::*, videoio};
// fn main() -> Result<()> {
//     println!("Hello, world!");
//     let mut cam = videoio::VideoCapture::from_file("rtsp://10.50.31.241/axis-media/media.amp",
//      videoio::CAP_GSTREAMER).unwrap(); // 0 is the default camera
//     let opened = videoio::VideoCapture::is_opened(&cam).unwrap();
// 	if !opened {
// 		panic!("Unable to open default camera!");
// 	}
//     let mut img_count = 0;
//     let mut count = 0;
//     let mut params = opencv::types::VectorOfi32::new();
//     let mut frame_buffer = opencv::types::VectorOfu8::new();
//     params.push(imgcodecs::IMWRITE_JPEG_QUALITY);
//     params.push(50);
//     loop {
// 		let mut frame = Mat::default();
// 		let is_read = cam.read(&mut frame)?;
//         if is_read {
//             //Save frame to jpg image.
//                 // imgcodecs::imwrite(format!("img-{}.jpg",img_count).as_str(), &frame, &params)?;
//                 imgcodecs::imencode(".jpg", &frame, &mut frame_buffer, &params)?;
//                 println!("frame buffer length: {}", frame_buffer.len());
//             //     let img_result =
//             //     image::load_from_memory_with_format(frame_buffer.as_slice(), image::ImageFormat::Jpeg);
//             // let img = match img_result {
//             //     Ok(image) => image,
//             //     Err(_) => {
//             //         println!("not image format");
//             //         return Ok(())
//             //     },
//             // };
//             // img.save(format!("img_bytes-{}.jpg", img_count)).unwrap();
//             // let img16 = img.into_rgb8();
//             // let data = img16.into_raw() as Vec<u8>;
//             // println!("Image length: {}", data.len());
//             count += 1;
//             count += 15;
//             let val = cam.get(videoio::CAP_PROP_POS_FRAMES)?;
//             println!("Get: {}", val);
//             cam.set(videoio::CAP_PROP_POS_FRAMES, count as f64)?;
//         } else {
//             cam.release();
//             break;
//         }
//         println!("End of 1 frame.");
//         img_count += 1;
// 		// let key = highgui::wait_key(10)?;
// 		// if key > 0 && key != 255 {
// 		// 	break;
// 		// }
// 	}
// 	Ok(())
// }

//FFMPEG-RUST
// extern crate ffmpeg_next as ffmpeg;

// use ffmpeg::format::{input, Pixel};
// use ffmpeg::media::Type;
// use ffmpeg::software::scaling::{context::Context, flag::Flags};
// use ffmpeg::util::frame::video::Video;
// use std::fs::File;
// use std::io::prelude::*;
// use std::thread;
// use tokio::runtime::Handle;

// #[tokio::main]
// async fn main() {
//     let handle = Handle::current();

//     let urls = [
//         // "rtsp://vietnam:L3xRay123!@10.50.30.212/1/h264major",
//         // "rtsp://10.50.29.36/1/h264major",
//         // "rtsp://10.50.31.171/1/h264major",
//         // "rtsp://vietnam:L3xRay123!@10.50.12.187/media/video1",
//         "rtsp://10.50.30.100/1/h264major",

//     ];

//     for url in urls {
//         handle.spawn(async move { get_frame(url).await });
//     }

//     loop {}
// }

// async fn get_frame(cam_url: &str) -> Result<(), ffmpeg::Error> {
//     ffmpeg::init().unwrap();

//     println!("{:?}", cam_url);

//     let path = cam_url.clone();
//     if let Ok(mut ictx) = input(&path) {
//         let input = ictx
//             .streams()
//             .best(Type::Video)
//             .ok_or(ffmpeg::Error::StreamNotFound)?;
//         let video_stream_index = input.index();

//         let mut decoder = input.codec().decoder().video()?;
//         //  println!("Codec: {:?}", input.codec());

//         let mut scaler = Context::get(
//             decoder.format(),
//             decoder.width(),
//             decoder.height(),
//             Pixel::RGB24,
//             decoder.width(),
//             decoder.height(),
//             Flags::BILINEAR,
//         )?;

//         let mut frame_index = 0;

//         let mut receive_and_process_decoded_frames =
//             |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
//                 let mut decoded = Video::empty();
//                 while decoder.receive_frame(&mut decoded).is_ok() {
//                     let mut rgb_frame = Video::empty();
//                     scaler.run(&decoded, &mut rgb_frame)?;
//                     // save_file(&rgb_frame, frame_index).unwrap();
//                     frame_index += 1;
//                 }
//                 Ok(())
//             };

//         for (stream, packet) in ictx.packets() {
//             if stream.index() == video_stream_index {
//                 decoder.send_packet(&packet)?;
//                 receive_and_process_decoded_frames(&mut decoder)?;
//             }
//         }
//         decoder.send_eof()?;
//         receive_and_process_decoded_frames(&mut decoder)?;
//     }
//     // loop{}
//     Ok(())
// }

// fn save_file(frame: &Video, index: usize) -> std::result::Result<(), std::io::Error> {
//     let mut file = File::create(format!("frame{}.ppm", index))?;
//     file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
//     // file.write_all(frame.data(0))?;
//     let data = frame.data(0);
//     let stride = frame.stride(0);
//     let byte_width: usize = 3 * frame.width() as usize;
//     let height: usize = frame.height() as usize;
//     for line in 0..height {
//         let begin = line * stride;
//         let end = begin + byte_width;
//         file.write_all(&data[begin..end])?;
//     }
//     Ok(())
// }

//FFAV
// use clap::{App, Arg};
// use ffav::easy::{AVError, AVFrameOwned, SimpleDecoder, SimpleReader};
// use ffav::ffi::{AVCodecID, AVPacket};
// use std::convert::TryInto;
// use std::sync::{
//     atomic::{AtomicBool, Ordering},
//     Arc,
// };
// use std::time::Instant;

// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // let matches = App::new("Simple Decoder")
//     //     .version("1.0")
//     //     .author("Varphone Wong <varphone@qq.com>")
//     //     .about("Example for SimpeDecoder")
//     //     .arg(
//     //         Arg::with_name("FILE")
//     //             .help("Sets the input file to use for decoding")
//     //             .required(true),
//     //     )
//     //     .get_matches();

//     // let input_file = matches
//     //     .value_of("FILE")
//     //     .ok_or("The input file must be specified")?;
//     let input_file = "rtsp://10.50.30.100/1/h264major";

//     let early_exit = Arc::new(AtomicBool::new(false));
//     let early_exit_cloned = Arc::clone(&early_exit);
//     let early_exit_thread = std::thread::spawn(move || {
//         let mut buffer = String::new();
//         std::io::stdin().read_line(&mut buffer).unwrap();
//         early_exit_cloned.store(true, Ordering::SeqCst);
//     });

//     ffav::util::error::register_all();

//     let mut decoder = SimpleDecoder::new(AVCodecID::AV_CODEC_ID_H264)?;
//     let mut reader = SimpleReader::open(input_file, None, None)?;
//     let stream_codecs: Vec<AVCodecID> = reader
//         .streams()
//         .iter()
//         .map(|x| match x.codecpar() {
//             Some(v) => v.codec_id,
//             None => AVCodecID::AV_CODEC_ID_NONE,
//         })
//         .collect();

//     println!("streams()={:#?}", reader.streams());
//     for s in reader.streams() {
//         println!("codecpar={:#?}", s.codecpar().unwrap());
//     }

//     let mut frame = AVFrameOwned::new()?;

//     for (mut packet, _info) in reader.frames() {
//         if early_exit.load(Ordering::SeqCst) {
//             break;
//         }

//         let bytes =
//             unsafe { std::slice::from_raw_parts(packet.data, packet.size.try_into().unwrap()) };
//         let n = bytes.len().min(16);
//         if stream_codecs[packet.stream_index as usize] == AVCodecID::AV_CODEC_ID_H264 {
//             match decoder.receive_frame(&mut frame) {
//                 Ok(_) => {
//                     // println!("Frame {:#?}", frame)

//             },
//                 Err(e) => match e {
//                     AVError::Again => { // Try next
//                     }
//                     e => println!("Error: {:?}", e),
//                 },
//             }
//             let r = decoder.send_packet(&mut packet);
//             // println!("r2={:?}", r);
//         }
//     }

//     early_exit_thread.join().unwrap();

//     Ok(())
// }

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

fn create_pipeline(uri: String, out_path: std::path::PathBuf) -> Result<gst::Pipeline, Error> {
    gst::init()?;

    // Create our pipeline from a pipeline description string.
    // let pipeline = gst::parse_launch(&format!(
    //     "rtspsrc location={} latency=0 ! queue ! rtpjitterbuffer ! rtph264depay ! queue ! h264parse ! vaapih264dec ! queue ! videorate ! videoconvert ! videoscale ! jpegenc !  appsink name=sink ",
    //     uri
    // ))?
    let pipeline = gst::parse_launch(&format!(
        "rtspsrc location={} latency=0 ! rtpjitterbuffer ! rtph264depay ! h264parse ! vaapih264dec ! videorate ! videoconvert ! videoscale ! jpegenc ! appsink name=sink ",
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
    appsink.set_caps(Some(
        &gst::Caps::builder("video/x-raw")
            .field("framerate", "5/1".to_string())
            .build(),
    ));
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

                //SAVE IMAGE
                let img = 
                image::load_from_memory_with_format(map.as_slice(), ImageFormat::Jpeg).unwrap();
                // let img = match img_result {
                //     Ok(image) => image,
                //     Err(_) => ,
                // };
                img.save(format!("img-{}", count)).unwrap();
                let img16 = img.into_rgb8();
                let data = img16.into_raw() as Vec<u8>;
                println!("Image length: {}", data.len());
                count += 1;

                // println!("xxxxxxxx Map {:?}", map);

                // let samples = map.as_slice_of::<u8>().map_err(|_| {
                //     element_error!(
                //         appsink,
                //         gst::ResourceError::Failed,
                //         ("Failed to interprete buffer as S16 PCM")
                //     );

                //     gst::FlowError::Error
                // })?;


                // Make sure that we only get a single buffer
                // if got_snapshot {
                //     return Err(gst::FlowError::Eos);
                // }
                // got_snapshot = true;

            //     let caps = sample.caps().expect("Sample without caps");
            //     let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");
            //     println!("info: {:?}", info);
            //     // // At this point, buffer is only a reference to an existing memory region somewhere.
            //     // // When we want to access its content, we have to map it while requesting the required
            //     // // mode of access (read, read/write).
            //     // // This type of abstraction is necessary, because the buffer in question might not be
            //     // // on the machine's main memory itself, but rather in the GPU's memory.
            //     // // So mapping the buffer makes the underlying memory region accessible to us.
            //     // // See: https://gstreamer.freedesktop.org/documentation/plugin-development/advanced/allocation.html
            //     let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
            //         .map_err(|_| {
            //             element_error!(
            //                 appsink,
            //                 gst::ResourceError::Failed,
            //                 ("Failed to map buffer readable")
            //             );

            //             gst::FlowError::Error
            //         })?;

            //     // We only want to have a single buffer and then have the pipeline terminate
            //     println!("Have video frame: {:?}", frame.buffer());
                

            //     // Calculate a target width/height that keeps the display aspect ratio while having
            //     // a height of 240 pixels
            //     // let display_aspect_ratio = (frame.width() as f64 * *info.par().numer() as f64)
            //     //     / (frame.height() as f64 * *info.par().denom() as f64);
            //     // let target_height = 240;
            //     // let target_width = target_height as f64 * display_aspect_ratio;

            //     // Create a FlatSamples around the borrowed video frame data from GStreamer with
            //     // the correct stride as provided by GStreamer.
            //     // let img = image::FlatSamples::<&[u8]> {
            //     //     samples: frame.plane_data(0).unwrap(),
            //     //     layout: image::flat::SampleLayout {
            //     //         channels: 3,       // RGB
            //     //         channel_stride: 1, // 1 byte from component to component
            //     //         width: frame.width(),
            //     //         width_stride: 4, // 4 byte from pixel to pixel
            //     //         height: frame.height(),
            //     //         height_stride: frame.plane_stride()[0] as usize, // stride from line to line
            //     //     },
            //     //     color_hint: Some(image::ColorType::Rgb8),
            //     // };

            //     //SAVING IMAGE
            //     let img = 
            //     image::load_from_memory_with_format(&frame.plane_data(0).unwrap(), ImageFormat::Jpeg).unwrap();
            // // let img = match img_result {
            // //     Ok(image) => image,
            // //     Err(_) => ,
            // // };
            //     img.save(format!("img-{}", count)).unwrap();
            //     let img16 = img.into_rgb8();
            //     let data = img16.into_raw() as Vec<u8>;
            //     println!("Image length: {}", data.len());
            //     count += 1;

                // // let img_buffer = img.as_slice();                

                // // // Scale image to our target dimensions
                // let scaled_img = image::imageops::thumbnail(
                //     &img.as_view::<image::Rgb<u8>>()
                //         .expect("couldn't create image view"),
                //         target_width as u32,
                //     target_height as u32,
                // );

                // // // Save it at the specific location. This automatically detects the file type
                // // // based on the filename.
                // scaled_img.save(&out_path).map_err(|err| {
                //     element_error!(
                //         appsink,
                //         gst::ResourceError::Write,
                //         (
                //             "Failed to write thumbnail file {}: {}",
                //             out_path.display(),
                //             err
                //         )
                //     );

                //     gst::FlowError::Error
                // })?;

                // println!("Wrote thumbnail to {}", out_path.display());

                Ok(gst::FlowSuccess::Ok)
                // Err(gst::FlowError::Error)
            })
            .build(),
    );

    Ok(pipeline)
}

fn main_loop(pipeline: gst::Pipeline, position: u64) -> Result<(), Error> {
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

fn main() {
    // use std::env;

    // let mut args = env::args();

    // Parse commandline arguments: input URI, position in seconds, output path
    // let _arg0 = args.next().unwrap();
    // let uri = args
    //     .next()
    //     .expect("No input URI provided on the commandline");
    // let position = args
    //     .next()
    //     .expect("No position in second on the commandline");
    // let position = position
    //     .parse::<u64>()
    //     .expect("Failed to parse position as integer");
    // let out_path = args
    //     .next()
    //     .expect("No output path provided on the commandline");
    let uri = "rtsp://10.50.30.100/1/h264major".to_owned();
    let position = 10;
    let out_path = "/img";
    let out_path = std::path::PathBuf::from(out_path);

    match create_pipeline(uri, out_path).and_then(|pipeline| main_loop(pipeline, position)) {
        Ok(r) => r,
        Err(e) => eprintln!("Error! {}", e),
    }
}
