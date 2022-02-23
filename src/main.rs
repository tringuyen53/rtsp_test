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
use std::sync::mpsc::channel;
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

 fn create_pipeline(id: String, uri: String, client: Connection, is_frame_getting: Arc<Mutex<bool>>,is_record: Arc<Mutex<bool>>,
    is_live: Arc<Mutex<bool>>,
    width: Arc<Mutex<usize>>,
    height: Arc<Mutex<usize>>,) -> Result<gst::Pipeline, Error> {
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
        "rtspsrc location={} !
        application/x-rtp, media=video, encoding-name=H264!
        rtph264depay ! queue leaky=2 !
        h264parse ! tee name=thumbnail_video !
        queue leaky=2 ! vaapih264dec !
        videorate ! video/x-raw, framerate=3/1 !
        vaapipostproc ! vaapijpegenc !
        appsink name=app1 max-buffers=100 emit-signals=false drop=true
        thumbnail_video. ! queue leaky=2 ! vaapih264dec !
        videorate ! video/x-raw, framerate=3/1 !
        vaapipostproc ! video/x-raw, width=720, height=480 ! vaapijpegenc !
        appsink name=app2 max-buffers=100 emit-signals=false drop=true" ,
        uri
    ))?
    .downcast::<gst::Pipeline>()
    .expect("Expected a gst::Pipeline");

    println!("pipeline: {:?} - {:?}", uri, pipeline);
    // Get access to the appsink element.
    let appsink_full = pipeline
        .by_name("app1")
        .expect("Sink element not found")
        .downcast::<gst_app::AppSink>()
        .expect("Sink element is expected to be an appsink!");

    let appsink_thumb = pipeline
        .by_name("app2")
        .expect("Sink element not found")
        .downcast::<gst_app::AppSink>()
        .expect("Sink element is expected to be an appsink!");

    // let pipeline = gst::Pipeline::new(None);
    // let src = gst::ElementFactory::make("rtspsrc", None)
    //     .map_err(|_| MissingElement("rtspsrc"))?;
    // src.set_property("location", &uri);
    // let src = gst::ElementFactory::make("videotestsrc", None)
    //     .map_err(|_| MissingElement("videotestsrc"))?;

    // let rtph264depay = gst::ElementFactory::make("rtph264depay", None)
    //     .map_err(|_| MissingElement("rtph264depay"))?;
    // let queue = gst::ElementFactory::make("queue", Some("queue"))
    //     .expect("Could not create queue element.");
    // queue.set_property_from_str("leaky", "upstream");
    // let queue_2 = gst::ElementFactory::make("queue", Some("queue_2"))
    //     .expect("Could not create queue element.");
    // queue_2.set_property_from_str("leaky", "upstream");
    // let queue_3 = gst::ElementFactory::make("queue", Some("queue_3"))
    //     .map_err(|_| MissingElement("queue"))?;
    // let h264parse = gst::ElementFactory::make("h264parse", None)
    //     .map_err(|_| MissingElement("h264parse"))?;
    // let vaapih264dec = gst::ElementFactory::make("vaapih264dec", None)
    //     .map_err(|_| MissingElement("vaapih264dec"))?;
    // let videorate = gst::ElementFactory::make("videorate", None)
    //     .map_err(|_| MissingElement("videorate"))?;
    // let vaapipostproc = gst::ElementFactory::make("vaapipostproc", None)
    //     .map_err(|_| MissingElement("vaapipostproc"))?;
    // let vaapijpegenc = gst::ElementFactory::make("vaapijpegenc", None)
    //     .map_err(|_| MissingElement("vaapijpegenc"))?;
    
    
    // let sink = gst::ElementFactory::make("appsink", None).map_err(|_| MissingElement("appsink"))?;
    // // sink.set_property("max-buffer", 100.to_value());
    // // sink.set_property("emit-signals", false);
    // // sink.set_property("drop", true);
    // println!("Before add_many");
    // // pipeline.add_many(&[&src, &rtph264depay, &queue, &h264parse, &queue_2, &vaapih264dec, &videorate, &queue_3, &vaapipostproc, &vaapijpegenc, &sink])?;
    // pipeline.add_many(&[&src, &sink])?;
    // println!("After add_many");
    // // gst::Element::link_many(&[&src, &sink])?;
    // src.connect_pad_added(f)
    // let res = src.link(&sink);
    // match res {
    //     Ok(()) => {
    //         println!("Link success");
    //     },
    //     Err(e) => {
    //         println!("Error: {:?}", e);
    //     }
    // }
    println!("pipeline: {:?} - {:?}", uri, pipeline);

    // let appsink = sink
    //     .dynamic_cast::<gst_app::AppSink>()
    //     .expect("Sink element is expected to be an appsink!");


    let mut count_full = 1;
    let mut count_thumb = 1;
    let mut got_snapshot = false;
    // Getting data out of the appsink is done by setting callbacks on it.
    // The appsink will then call those handlers, as soon as data is available.
    let id_1 = id.clone();
    let pipeline_weak = pipeline.downgrade();
    // let is_frame_getting_weak = Arc::downgrade(&is_frame_getting);
    let is_frame_getting_2 = is_frame_getting.clone();
    appsink_full.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            // Add a handler to the "new-sample" signal.
            .new_sample(move |appsink_full| {
                // let is_live_bool = *is_live.lock().unwrap();
                // let is_record_bool = *is_record.lock().unwrap();
                // let frame_width = *width.lock().unwrap();
                // let frame_height = *height.lock().unwrap();
                // let is_frame_getting =  is_frame_getting.clone();
                
                // Pull the sample in question out of the appsink's buffer.
                let sample = appsink_full.pull_sample().map_err(|_| gst::FlowError::Eos)?;
            //    println!("Sample: {:?}", sample);
                let buffer = sample.buffer().ok_or_else(|| {
                    element_error!(
                        appsink_full,
                        gst::ResourceError::Failed,
                        ("Failed to get buffer from appsink")
                    );

                    gst::FlowError::Error
                })?;

                // if let Some(is_frame_getting) = is_frame_getting_weak.upgrade() {
                    if !*is_frame_getting_2.lock().unwrap() {
                        println!("Send EOS.....");
                        appsink_full.send_event(gst::event::Eos);
                        // if let Some(pipeline) = pipeline_weak.upgrade() {
                        //     println!("Pipeline after upgrade: {:?}", pipeline);
                        //     let ev = gst::event::Eos::new();
                        //     let pipeline_weak = pipeline_weak.clone();
                        //         if let Some(pipeline) = pipeline_weak.upgrade() {
                        //             // let res = pipeline.send_event(ev);
                        //             pipeline.set_state(gst::State::Null);
                        //             // println!("send event: {}", res);
                        //         }
                        // }
                    }
                    // return Err(gst::FlowError::Eos);
                // }      

        //        println!("Buffer {:?}", buffer);
        // if count == 50 {
        //     println!("stop pipeline");
        //     *is_frame_getting.lock().unwrap() = false;
        //     // drop(is_frame_getting.lock().unwrap());
        //     return Err(gst::FlowError::Eos);
        // }

                let map = buffer.map_readable().map_err(|_| {
                    element_error!(
                        appsink_full,
                        gst::ResourceError::Failed,
                        ("Failed to map buffer readable")
                    );

                    gst::FlowError::Error
                })?;
  //              println!("xxxxxxxx Map {:?}", map);   

                let samples = map.as_slice_of::<u8>().map_err(|_| {
                    element_error!(
                        appsink_full,
                        gst::ResourceError::Failed,
                       ("Failed to interprete buffer as S16 PCM")
                    );

                    gst::FlowError::Error
                })?;
                // println!("{:?}",samples.len());
                 //SAVE IMAGE
                //  let mut file = fs::File::create(format!("packet-{}", count)).unwrap();
                //  file.write_all(samples);

                // let origin_img_result = 
                //     image::load_from_memory_with_format(samples, ImageFormat::Jpeg);
                // match origin_img_result {
                //     Ok(image) => {
                //             image.save(format!("full-img-{}-{}.jpg", id_1, count_full)).unwrap();
                //             count_full += 1;
                //     },
                //     Err(e) => {
                //         println!("origin load image error: {:?}", e);
                //         ()
                //     },
                // };

                // let caps = sample.caps().expect("Sample without caps");
                // let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");
                // println!("Info: {:?}", info);

                // let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
                //     .map_err(|_| {
                //         element_error!(
                //             appsink,
                //             gst::ResourceError::Failed,
                //             ("Failed to map buffer readable")
                //         );

                //         gst::FlowError::Error
                //     })?;


            //     let new_image = image::load_from_memory_with_format(samples, ImageFormat::Jpeg);
            //     let new_image = match new_image { 
            //         Ok(image) => {
            //         let width = NonZeroU32::new(image.width()).unwrap();
            //         let height = NonZeroU32::new(image.height()).unwrap();
            //         // println!("Origin width height - {:?}x{:?} - color type: {:?}", width, height, image.color());

            //         // let test_into_raw_image =  image::load_from_memory_with_format(&image.to_rgb8().into_raw(), ImageFormat::Jpeg);
            //         // match test_into_raw_image {
            //         //     Ok(image) => {
            //         //         image.save(format!("test-load-rgb8-img-{}-{}.jpg", seed, count)).unwrap();
            //         //      count += 1;
            //         //     },
            //         //     Err(e) => {
            //         //         println!("test load rgb8 image error: {:?}", e);
            //         //         ()
            //         //     },
            //         // };

            //         let mut src_image = fr::Image::from_vec_u8(
            //             width,
            //             height,
            //             image.to_rgb8().into_raw(),
            //             fr::PixelType::U8x3
            //         ).unwrap();

            //         // let origin_after_torgba8_img_result = 
            //         // image::load_from_memory_with_format(src_image.buffer(), ImageFormat::Jpeg);
            //         // match origin_after_torgba8_img_result {
            //         //     Ok(image) => {
            //         //             image.save(format!("origin-rgba8-img-{}-{}.jpg", seed, count)).unwrap();
            //         //         //  count += 1;
            //         //     },
            //         //     Err(e) => {
            //         //         println!("scaled load image error: {:?}", e);
            //         //         ()
            //         //     },
            //         // };

            //         // let alpha_mul_div = fr::MulDiv::default();
            //         // alpha_mul_div.multiply_alpha_inplace(&mut src_image.view_mut()).unwrap();

            //         let dst_width = NonZeroU32::new(720).unwrap();
            //         let dst_height = NonZeroU32::new(540).unwrap();

            //         let mut dst_image = fr::Image::new(
            //             dst_width,
            //             dst_height,
            //             src_image.pixel_type(),
            //         );

            //         let mut dst_view = dst_image.view_mut();

            //         let mut resizer = fr::Resizer::new(
            //             fr::ResizeAlg::Convolution(fr::FilterType::Box)
            //         );

            //         resizer.resize(&src_image.view(), &mut dst_view).unwrap();

            //         // alpha_mul_div.divide_alpha_inplace(&mut dst_view).unwrap();
                    
            //         let mut result_buf = BufWriter::new(Vec::new());
            //         image::codecs::jpeg::JpegEncoder::new(&mut result_buf).encode(dst_image.buffer(), dst_width.get(), dst_height.get(), ColorType::Rgb8).unwrap();

            //         Vec::from(result_buf.into_inner().unwrap())
            //     }
            //     Err(_) => unreachable!(),
            // };
            println!("[FULL] Timestamp: {:?} - cam_id: {:?}", std::time::SystemTime::now(), id_1);
            count_full += 1;
            if count_full == 10 {
                println!("Stop pipeline");
                *is_frame_getting.lock().unwrap() = false;
                let cam_dist = Distributor::named(format!("rtsp-{}", id_1.clone()));
                cam_dist.tell_one(id_1.clone()).expect("Send stop failed.");
            }
        //      let img_result = 
        //          image::load_from_memory_with_format(&new_image, ImageFormat::Jpeg);
        //      match img_result {
        //          Ok(image) => {
        //                 // println!("WxH after scale: {:?}x{:?}", image.width(), image.height());
        //                 //  image.save(format!("final-img-{}-{}.jpg", id, count)).unwrap();
        //                  count += 1;
        //             },
        //          Err(e) => {
		// 	println!("final load image error: {:?}", e);
		// 	()
		// },
        //      };
             
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
    let id_2 = id.clone();
    appsink_thumb.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            // Add a handler to the "new-sample" signal.
            .new_sample(move |appsink_thumb| {
                // let is_live_bool = *is_live.lock().unwrap();
                // let is_record_bool = *is_record.lock().unwrap();
                // let frame_width = *width.lock().unwrap();
                // let frame_height = *height.lock().unwrap();
                let id = id.clone();
                // Pull the sample in question out of the appsink's buffer.
                let sample = appsink_thumb.pull_sample().map_err(|_| gst::FlowError::Eos)?;
            //    println!("Sample: {:?}", sample);
                let buffer = sample.buffer().ok_or_else(|| {
                    element_error!(
                        appsink_thumb,
                        gst::ResourceError::Failed,
                        ("Failed to get buffer from appsink")
                    );

                    gst::FlowError::Error
                })?;

        //        println!("Buffer {:?}", buffer);
        // if count == 50 {
        //     println!("stop pipeline");
        //     *is_frame_getting.lock().unwrap() = false;
        //     // drop(is_frame_getting.lock().unwrap());
        //     return Err(gst::FlowError::Eos);
        // }

                let map = buffer.map_readable().map_err(|_| {
                    element_error!(
                        appsink_thumb,
                        gst::ResourceError::Failed,
                        ("Failed to map buffer readable")
                    );

                    gst::FlowError::Error
                })?;
  //              println!("xxxxxxxx Map {:?}", map);   

                let samples = map.as_slice_of::<u8>().map_err(|_| {
                    element_error!(
                        appsink_thumb,
                        gst::ResourceError::Failed,
                       ("Failed to interprete buffer as S16 PCM")
                    );

                    gst::FlowError::Error
                })?;
                // println!("{:?}",samples.len());
                 //SAVE IMAGE
                //  let mut file = fs::File::create(format!("packet-{}", count)).unwrap();
                //  file.write_all(samples);

                // let origin_img_result = 
                //     image::load_from_memory_with_format(samples, ImageFormat::Jpeg);
                // match origin_img_result {
                //     Ok(image) => {
                //             image.save(format!("thumb-img-{}-{}.jpg", id_2, count_thumb)).unwrap();
                //          count_thumb += 1;
                //     },
                //     Err(e) => {
                //         println!("origin load image error: {:?}", e);
                //         ()
                //     },
                // };

                // let caps = sample.caps().expect("Sample without caps");
                // let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");
                // println!("Info: {:?}", info);
                println!("[THUMB] Timestamp: {:?} - cam_id: {:?}", std::time::SystemTime::now(), id_2);
            // count += 1;

        //      let img_result = 
        //          image::load_from_memory_with_format(&new_image, ImageFormat::Jpeg);
        //      match img_result {
        //          Ok(image) => {
        //                 // println!("WxH after scale: {:?}x{:?}", image.width(), image.height());
        //                 //  image.save(format!("final-img-{}-{}.jpg", id, count)).unwrap();
        //                  count += 1;
        //             },
        //          Err(e) => {
		// 	println!("final load image error: {:?}", e);
		// 	()
		// },
        //      };
             
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
    
    println!("End Of Pipeline");
    Ok(pipeline)
}

fn main_loop(pipeline: gst::Pipeline, id: String, is_frame_getting: Arc<Mutex<bool>>,) -> Result<(), Error> {
    println!("Start main loop");
    pipeline.set_state(gst::State::Playing)?;

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

//    println!("Bus: {:?}", bus);

// let mut seeked = false;

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        println!("In loop msg: {:?}", msg);
        use gst::MessageView;
        // println!("is getting frame: {}",*is_frame_getting.lock().unwrap());
        
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
                println!("{:?} - Error: {:?}",id, err.error());
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
        // "rtsp://10.50.31.171/1/h264major",
        // "rtsp://10.50.31.172/1/h264major",
        // "rtsp://10.50.13.231/1/h264major",
        // "rtsp://10.50.13.233/1/h264major",
        // "rtsp://10.50.13.234/1/h264major",
        // "rtsp://10.50.13.235/1/h264major",
        // "rtsp://10.50.13.236/1/h264major",
        // "rtsp://10.50.13.237/1/h264major",
        // "rtsp://10.50.13.238/1/h264major",
        // "rtsp://10.50.13.239/1/h264major",
        // "rtsp://10.50.13.240/1/h264major",
        // "rtsp://10.50.13.241/1/h264major",
        // "rtsp://10.50.13.242/1/h264major",
        // "rtsp://10.50.13.243/1/h264major",
        // "rtsp://10.50.13.244/1/h264major",
        // "rtsp://10.50.13.245/1/h264major",
        // "rtsp://10.50.13.248/1/h264major",
        // "rtsp://10.50.13.249/1/h264major",
        // "rtsp://10.50.13.250/1/h264major",
        // "rtsp://10.50.13.251/1/h264major",
        // "rtsp://10.50.13.252/1/h264major",
        // "rtsp://10.50.13.253/1/h264major",
        "rtsp://10.50.13.254/1/h264major",
    ];

    Bastion::init();

    let cam_ip = vec![
        // 36,
        // 171,
        // 172, 
        // 231, 
        // 233, 
        // 234, 
        // 235, 
        // 236, 
        // 237, 
        // 238, 
        // 239, 
        // 240,
        // 241,
        // 242, 
        // 243, 
        // 244, 
        //245, 
        //248, 
        //249,
        // 250, 
        // 251,
        //252, 
        //253, 
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
    let is_record = Arc::new(Mutex::new(false));
    let is_live = Arc::new(Mutex::new(false));
    let width = Arc::new(Mutex::new(720));
    let height = Arc::new(Mutex::new(480));
    loop {
        MessageHandler::new(ctx.recv().await?)
            .on_tell(|message: RTPMessage, _| {
//let mut rng = rand::thread_rng();                
//let n1: u8 = rng.gen();
println!("spawn new actor: {:?}", message.id);
// let rt = tokio::runtime::Runtime::new().unwrap();
    let is_frame_getting = is_frame_getting.clone();
    let is_record = is_record.clone();
    let is_live = is_live.clone();
    let width = width.clone();
    let height = height.clone();
    let id_2 = message.id.clone();
               rt.spawn_blocking( move || {  
                  create_pipeline(message.id, message.url, message.client, is_frame_getting.clone(), is_record.clone(), is_live.clone(), width.clone(), height.clone()).and_then(|pipeline| main_loop(pipeline, id_2, is_frame_getting.clone()));
//let pipeline = create_pipeline(message.to_owned(), n1).await.unwrap();
  //                  main_loop(pipeline)          
    });
    // handle.await?;
    // println!("Arc counter: {}", Arc::strong_count(&is_frame_getting));
            })
            .on_tell(|msg: String, _| {
                let child_ref = ctx.current().clone();
                        let cam_distributor_by_id =
                            Distributor::named(format!("rtsp-{}", msg));
                        cam_distributor_by_id.unsubscribe(child_ref).expect("unsub failed");
                        ctx.supervisor()
                            .unwrap()
                            .stop()
                            .expect("[CAMERA] Couldn't stop.");
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
