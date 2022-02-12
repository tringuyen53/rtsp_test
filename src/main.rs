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
use image::GenericImageView;
use std::io::Write; // bring trait into scope
use std::fs;
use tokio::runtime::Handle;
use rand::Rng;
use anyhow::Error;
use derive_more::{Display, Error};
use image::*;
use bastion::distributor::*;
use bastion::prelude::*;
mod throttle;
use throttle::Throttle;
use tokio::net::TcpStream;
use std::error::Error as OtherError;
use nats::{self, asynk::Connection};
use async_std::task;
use fast_image_resize as fr;
use std::io::BufWriter;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
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

#[derive(Debug, Clone)]
pub struct RTPMessage {
    pub url: String,
    pub client: Connection,
    pub id: String,
}

const NATS_URL: &str = "tls://dev.lexray.com:60064";
// async fn connect_nats() -> Connection {
//     println!("Connecting to NATS..");
//     nats::asynk::Options::with_credentials("hub.creds")
//         .connect(NATS_URL)
//         .await
//         .unwrap()
// }
async fn connect_nats() -> Connection {
    nats::asynk::connect("nats://demo.nats.io:4222")
        .await
        .unwrap()
}

 fn create_pipeline(id: String, uri: String, client: Connection, is_frame_getting: Arc<Mutex<bool>>,) -> Result<gst::Pipeline, Error> {
    // let client = task::block_on(connect_nats());
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
    //  let pipeline = gst::parse_launch(&format!(
    //      "rtspsrc location={} latency=300 !application/x-rtp, clock-rate=90000, encoding-name=H264, payload=96 ! rtpjitterbuffer latency=300 ! appsink name=sink max-buffers=100 emit-signals=false drop=true" ,
    //      uri
    //  ))?

     let pipeline = gst::parse_launch(&format!(
        "rtspsrc location={} ! rtph264depay ! queue leaky=2 ! h264parse ! queue leaky=2 ! vaapih264dec ! videorate ! video/x-raw,framerate=5/1 ! vaapipostproc ! vaapijpegenc ! appsink name=sink max-buffers=100 emit-signals=false drop=true" ,
        uri
    ))?
    .downcast::<gst::Pipeline>()
    .expect("Expected a gst::Pipeline");

    println!("pipeline: {:?} - {:?}", uri, pipeline);
    // Get access to the appsink element.
    let appsink = pipeline
        .by_name("sink")
        .expect("Sink element not found")
        .downcast::<gst_app::AppSink>()
        .expect("Sink element is expected to be an appsink!");

    let mut count = 0;
    let mut got_snapshot = false;
    // Getting data out of the appsink is done by setting callbacks on it.
    // The appsink will then call those handlers, as soon as data is available.
    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            // Add a handler to the "new-sample" signal.
            .new_sample(move |appsink| {
                // Pull the sample in question out of the appsink's buffer.
                let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
            //    println!("Sample: {:?}", sample);
                let buffer = sample.buffer().ok_or_else(|| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to get buffer from appsink")
                    );

                    gst::FlowError::Error
                })?;

        //        println!("Buffer {:?}", buffer);
        // if count == 5 {
        //     println!("stop pipeline");
        //     *is_frame_getting.lock().unwrap() = false;
        //     // return Err(gst::FlowError::Eos);
        // }

                let map = buffer.map_readable().map_err(|_| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to map buffer readable")
                    );

                    gst::FlowError::Error
                })?;
  //              println!("xxxxxxxx Map {:?}", map);   

                let samples = map.as_slice_of::<u8>().map_err(|_| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                       ("Failed to interprete buffer as S16 PCM")
                    );

                    gst::FlowError::Error
                })?;
                // println!("{:?}",samples);
                 //SAVE IMAGE
                //  let mut file = fs::File::create(format!("packet-{}", count)).unwrap();
                //  file.write_all(samples);

                // let origin_img_result = 
                //     image::load_from_memory_with_format(samples, ImageFormat::Jpeg);
                // match origin_img_result {
                //     Ok(image) => {
                //             image.save(format!("origin-img-{}-{}.jpg", seed, count)).unwrap();
                //         //  count += 1;
                //     },
                //     Err(e) => {
                //         println!("origin load image error: {:?}", e);
                //         ()
                //     },
                // };

                let caps = samples.caps().expect("Sample without caps");
                let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");
                println!("Info: {:?}", info);

                // let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
                //     .map_err(|_| {
                //         element_error!(
                //             appsink,
                //             gst::ResourceError::Failed,
                //             ("Failed to map buffer readable")
                //         );

                //         gst::FlowError::Error
                //     })?;


                let new_image = image::load_from_memory_with_format(samples, ImageFormat::Jpeg);
                let new_image = match new_image { 
                    Ok(image) => {
                    let width = NonZeroU32::new(image.width()).unwrap();
                    let height = NonZeroU32::new(image.height()).unwrap();
                    // println!("Origin width height - {:?}x{:?} - color type: {:?}", width, height, image.color());

                    // let test_into_raw_image =  image::load_from_memory_with_format(&image.to_rgb8().into_raw(), ImageFormat::Jpeg);
                    // match test_into_raw_image {
                    //     Ok(image) => {
                    //         image.save(format!("test-load-rgb8-img-{}-{}.jpg", seed, count)).unwrap();
                    //      count += 1;
                    //     },
                    //     Err(e) => {
                    //         println!("test load rgb8 image error: {:?}", e);
                    //         ()
                    //     },
                    // };

                    let mut src_image = fr::Image::from_vec_u8(
                        width,
                        height,
                        image.to_rgb8().into_raw(),
                        fr::PixelType::U8x3
                    ).unwrap();

                    // let origin_after_torgba8_img_result = 
                    // image::load_from_memory_with_format(src_image.buffer(), ImageFormat::Jpeg);
                    // match origin_after_torgba8_img_result {
                    //     Ok(image) => {
                    //             image.save(format!("origin-rgba8-img-{}-{}.jpg", seed, count)).unwrap();
                    //         //  count += 1;
                    //     },
                    //     Err(e) => {
                    //         println!("scaled load image error: {:?}", e);
                    //         ()
                    //     },
                    // };

                    // let alpha_mul_div = fr::MulDiv::default();
                    // alpha_mul_div.multiply_alpha_inplace(&mut src_image.view_mut()).unwrap();

                    let dst_width = NonZeroU32::new(720).unwrap();
                    let dst_height = NonZeroU32::new(540).unwrap();

                    let mut dst_image = fr::Image::new(
                        dst_width,
                        dst_height,
                        src_image.pixel_type(),
                    );

                    let mut dst_view = dst_image.view_mut();

                    let mut resizer = fr::Resizer::new(
                        fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3)
                    );

                    resizer.resize(&src_image.view(), &mut dst_view).unwrap();

                    // alpha_mul_div.divide_alpha_inplace(&mut dst_view).unwrap();
                    
                    let mut result_buf = BufWriter::new(Vec::new());
                    image::codecs::jpeg::JpegEncoder::new(&mut result_buf).encode(dst_image.buffer(), dst_width.get(), dst_height.get(), ColorType::Rgb8).unwrap();

                    Vec::from(result_buf.into_inner().unwrap())
                }
                Err(_) => unreachable!(),
            };

             let img_result = 
                 image::load_from_memory_with_format(&new_image, ImageFormat::Jpeg);
             match img_result {
                 Ok(image) => {
                        // println!("WxH after scale: {:?}x{:?}", image.width(), image.height());
                        //  image.save(format!("final-img-{}-{}.jpg", id, count)).unwrap();
                        //  count += 1;
                    },
                 Err(e) => {
			println!("final load image error: {:?}", e);
			()
		},
             };
             
            // let mut throttle = Throttle::new(std::time::Duration::from_secs(1), 1);
            // let result = throttle.accept();
            // if result.is_ok() {
                    // println!("Throttle START!!");
                    // count += 1;
                    // let transcode_actor = Distributor::named("transcode");
                    // transcode_actor.tell_one(samples.to_vec()).expect("Tell transcode failed");   
                    // let _ = client.publish(TOPIC, samples.to_vec());
                    drop(samples);
                    drop(map);
                    drop(buffer);
                    drop(sample);
                // }
                // println!("End of callbacks");
                Ok(gst::FlowSuccess::Ok)
                // Err(gst::FlowError::Eos)
            })
            .build(),
    );

    Ok(pipeline)
}

fn main_loop(pipeline: gst::Pipeline, is_frame_getting: Arc<Mutex<bool>>,) -> Result<(), Error> {
    println!("Start main loop");
    pipeline.set_state(gst::State::Playing)?;

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

//    println!("Bus: {:?}", bus);

let mut seeked = false;

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        // println!("In loop msg: {:?}", msg);
        use gst::MessageView;
        // println!("is getting frame: {}",*is_frame_getting.lock().unwrap());
        if !*is_frame_getting.lock().unwrap() {
            println!("Gudbaiiiiii");
            break;
        }
        match msg.view() {
            // MessageView::AsyncDone(..) => {
            //     if !seeked {
            //         // AsyncDone means that the pipeline has started now and that we can seek
            //         println!("Got AsyncDone message, seeking to {}s", 4);

            //         if pipeline
            //             .seek_simple(gst::SeekFlags::FLUSH, 4 * gst::ClockTime::SECOND)
            //             .is_err()
            //         {
            //             println!("Failed to seek, taking first frame");
            //         }

            //         pipeline.set_state(gst::State::Playing)?;
            //         seeked = true;
            //     } else {
            //         println!("Got second AsyncDone message, seek finished");
            //     }
            // },
            MessageView::Eos(..) => {
                // pipeline.set_state(gst::State::Null)?;
                println!("Got Eos message, done");
                break;
            },
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null)?;
                println!("Error: {:?}",err.error());
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
    println!("Main loop break");

    pipeline.set_state(gst::State::Null)?;

    Ok(())
}
#[tokio::main]
async fn main() {
    // let handle = Handle::current();
    

    let urls = [
        // "rtsp://10.50.29.36/1/h264major",
        // "rtsp://10.50.13.231/1/h264major",
        // "rtsp://10.50.13.233/1/h264major",
        // "rtsp://10.50.13.234/1/h264major",
        // "rtsp://10.50.13.235/1/h264major",
        // "rtsp://10.50.13.236/1/h264major",
        // "rtsp://10.50.13.237/1/h264major",
        // "rtsp://10.50.13.238/1/h264major",
        // "rtsp://10.50.13.239/1/h264major",
        "rtsp://10.50.13.240/1/h264major",
        "rtsp://10.50.13.241/1/h264major",
        // "rtsp://10.50.13.242/1/h264major",
        "rtsp://10.50.13.243/1/h264major",
        "rtsp://10.50.13.244/1/h264major",
        "rtsp://10.50.13.245/1/h264major",
        "rtsp://10.50.13.248/1/h264major",
        "rtsp://10.50.13.249/1/h264major",
        "rtsp://10.50.13.252/1/h264major",
        "rtsp://10.50.13.253/1/h264major",
        "rtsp://10.50.13.254/1/h264major",
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
    }).map_err(|_| println!("Error"));

    let cam_ip = vec![
        // 36, 
        // 231, 
        // 233, 
        // 234, 
        // 235, 
        // 236, 
        // 237, 
        // 238, 
        // 239, 
        240,
        241,
        // 242, 
        243, 
        244, 
        245, 
        248, 
        249, 
        252, 
        253, 
        254,
    ];

    for ip in &cam_ip {
        let name = format!("rtsp-{}", ip);
        Bastion::supervisor(|supervisor| {
            supervisor.children(|children| {
                // Iniit staff
                // Staff (5 members) - Going to organize the event
                children
                    .with_distributor(Distributor::named(name))
                    .with_exec(get_rtsp_stream)
            })
        }).map_err(|_| println!("Error"));
    }

    Bastion::start();
    std::thread::sleep(std::time::Duration::from_secs(2));

    let mut index = 0;
    let client = task::block_on(connect_nats());
    for ip in &cam_ip {
        let name = format!("rtsp-{}", ip);
        let rtsp_actor = Distributor::named(name);
        let msg = RTPMessage {
            url: urls[index].to_owned(),
            client: client.clone(),
            id: ip.to_string(),
        };
        rtsp_actor.tell_one(msg).expect("tell failed");
        index += 1;
    }
    
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
    let is_frame_getting = Arc::new(Mutex::new(true));
    loop {
        MessageHandler::new(ctx.recv().await?)
            .on_tell(|message: RTPMessage, _| {
//let mut rng = rand::thread_rng();                
//let n1: u8 = rng.gen();
//println!("spawn new actor: {:?} - {:?}", message, n1);
let is_frame_getting = is_frame_getting.clone();
                rt.spawn_blocking( move || {  
                  create_pipeline(message.id, message.url, message.client, is_frame_getting.clone()).and_then(|pipeline| main_loop(pipeline, is_frame_getting.clone()));
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
