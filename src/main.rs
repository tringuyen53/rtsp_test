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
use gst::SeekFlags;
use gst::SeekType;
use gst::element_error;
use gst::format::Undefined;
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
use bastion::distributor::*;
use bastion::prelude::*;
mod throttle;
use throttle::Throttle;
use async_std::task;
use nats::{self, asynk::Connection};
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
    // pub client: Connection,
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

 fn create_pipeline(id: String, uri: String, 
    // client: Connection
) -> Result<gst::Pipeline, Error> {
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
//RTP PACKET
    //  let pipeline = gst::parse_launch(&format!(
    //      "rtspsrc location={} latency=300 !application/x-rtp, clock-rate=90000, encoding-name=H264, payload=96 ! rtpjitterbuffer latency=300 ! appsink name=sink max-buffers=100 emit-signals=false drop=true" ,
    //      uri
    //  ))?
    // let pipeline = gst::parse_launch(&format!(
    //     "rtspsrc location={} latency=100 ! queue ! rtpjitterbuffer ! rtph264depay ! queue ! h264parse ! vaapih263dec ! queue ! videoconvert ! videoscale ! jpegenc ! appsink name=sink" ,
    //     uri
    // ))?
    // let pipeline = gst::parse_launch(&format!(
    //     "rtspsrc location={} name=src !
    //     application/x-rtp, media=video, encoding-name=H264!
    //     rtph264depay ! queue leaky=2 !
    //     h264parse ! tee name=thumbnail_video !
    //     queue leaky=2 ! vaapih264dec !
    //     videorate ! video/x-raw, framerate=3/1 !
    //     vaapipostproc ! video/x-raw, width=1920, height=1080 ! vaapijpegenc !
    //     appsink name=app1 max-buffers=100 emit-signals=false drop=true sync=false
    //     thumbnail_video. ! queue leaky=2 ! vaapih264dec !
    //     videorate ! video/x-raw, framerate=3/1 !
    //     vaapipostproc ! video/x-raw, width=720, height=480 ! vaapijpegenc !
    //     appsink name=app2 max-buffers=100 emit-signals=false drop=true sync=false",
    //     uri,
    // ))?
        //MJPEG
    // let pipeline = gst::parse_launch(&format!(
    //     "souphttpsrc location={} is-live=true do-timestamp=true ! jpegparse ! vaapijpegdec ! tee name=thumbnail_video ! queue leaky=2 !
    //     videorate ! video/x-raw, framerate=3/1 ! vaapijpegenc ! appsink name=app1 emit-signals=false drop=true sync=false
    //     thumbnail_video. ! queue leaky=2 ! 
    //     videorate ! video/x-raw, framerate=3/1 ! vaapipostproc ! video/x-raw, width=720, height=480 ! vaapijpegenc ! appsink name=app2 emit-signals=false drop=true sync=false" ,
    //     uri
    // ))?
    // .downcast::<gst::Pipeline>()
    // .expect("Expected a gst::Pipeline");

    // println!("pipeline: {:?} - {:?}", uri, pipeline);
    
    // Get access to the appsink element.
    // let appsink1 = pipeline
    //     .by_name("app1")
    //     .expect("Sink element not found")
    //     .downcast::<gst_app::AppSink>()
    //     .expect("Sink element is expected to be an appsink!");

    // let appsink2 = pipeline
    //     .by_name("app2")
    //     .expect("Sink element not found")
    //     .downcast::<gst_app::AppSink>()
    //     .expect("Sink element is expected to be an appsink!");

    let pipeline = gst::Pipeline::new(None)
        .downcast::<gst::Pipeline>()
        .expect("Expected a gst::Pipeline");
    // Initialize RTSP source
    let src = gst::ElementFactory::make("rtspsrc", None).map_err(|_| MissingElement("rtspsrc"))?;
    let rtph264depay = gst::ElementFactory::make("rtph264depay", Some("depay"))?;
    // Initialize h264parse
    let h264parse =
        gst::ElementFactory::make("h264parse", None).map_err(|_| MissingElement("h264parse"))?;
    // Initialize tee
    let tee = gst::ElementFactory::make("tee", Some("tee"))?;
    // Initialize queue 2
    let queue_2 =
        gst::ElementFactory::make("queue", Some("queue_2")).map_err(|_| MissingElement("queue"))?;
    // Initialize queue 3
    let queue_3 =
        gst::ElementFactory::make("queue", Some("queue_3")).map_err(|_| MissingElement("queue"))?;
    // Initialize queue 4
    let queue_4 =
        gst::ElementFactory::make("queue", Some("queue_4")).map_err(|_| MissingElement("queue"))?;
    // Initialize queue 5
    let queue_5 =
        gst::ElementFactory::make("queue", Some("queue_5")).map_err(|_| MissingElement("queue"))?;
    // Initialize queue 6
    let queue_6 =
        gst::ElementFactory::make("queue", Some("queue_6")).map_err(|_| MissingElement("queue"))?;
    // Initialize vaapijpegdec
    let vaapih264dec = gst::ElementFactory::make("vaapih264dec", None)
        .map_err(|_| MissingElement("vaapijpegdec"))?;
    // Initialize videorate
    let videorate = gst::ElementFactory::make("videorate", Some("videorate"))
        .map_err(|_| MissingElement("videorate"))?;
    // Initialize videorate_2
    let videorate_2 = gst::ElementFactory::make("videorate", Some("videorate_2"))
        .map_err(|_| MissingElement("videorate"))?;
    // Initialize videorate_3
    let videorate_3 = gst::ElementFactory::make("videorate", Some("videorate_3"))
    .map_err(|_| MissingElement("videorate"))?;
    // Initialize capsfilter for videorate
    let capsfilter = gst::ElementFactory::make("capsfilter", Some("capsfilter"))
        .map_err(|_| MissingElement("capsfilter"))?;
    let caps = gst::Caps::builder("video/x-raw")
        .field("framerate", gst::Fraction::new(5, 1))
        .build();
    // Initialize capsfilter for vaapipostproc
    let capsfilter_2 = gst::ElementFactory::make("capsfilter", Some("capsfilter_2"))
        .map_err(|_| MissingElement("capsfilter"))?;
    let caps_2 = gst::Caps::new_simple("video/x-raw", &[("width", &(1920 as i32)), ("height", &(1080 as i32))]);
    // Initialize capsfilter for videorate_2
    let capsfilter_3 = gst::ElementFactory::make("capsfilter", Some("capsfilter_3"))?;
    let caps_3 = gst::Caps::builder("video/x-raw")
        .field("framerate", gst::Fraction::new(3, 1))
        .build();
    // Initialize capsfilter for vaapipostproc_2
    let capsfilter_4 = gst::ElementFactory::make("capsfilter", Some("capsfilter_4"))?;
    let caps_4 = gst::Caps::builder("video/x-raw")
        .field("width", 720 as i32)
        .field("height", 480 as i32)
        .build();
    // Initialize capsfilter for videorate_3
    let capsfilter_5 = gst::ElementFactory::make("capsfilter", Some("capsfilter_5"))?;
    let caps_5 = gst::Caps::builder("video/x-raw")
        .field("framerate", gst::Fraction::new(3, 1))
        .build();
    // Initialize capsfilter for vaapipostproc_3
    let capsfilter_6 = gst::ElementFactory::make("capsfilter", Some("capsfilter_6"))?;
    let caps_6 = gst::Caps::new_simple("video/x-raw", &[("width", &(1920 as i32)), ("height", &(1080 as i32))]);
    // Initialize vaapipostproc
    let vaapipostproc = gst::ElementFactory::make("vaapipostproc", Some("vaapipostproc"))
        .map_err(|_| MissingElement("vaapipostproc"))?;
    // Initialize vaapijpegenc
    let vaapijpegenc = gst::ElementFactory::make("vaapijpegenc", Some("vaapijpegenc"))
        .map_err(|_| MissingElement("vaapijpegenc"))?;
    // Initialize appsink 1
    let sink = gst::ElementFactory::make("appsink", Some("sink"))
        .map_err(|_| MissingElement("appsink"))?;
    // Initialize vaapipostproc_2
    let vaapipostproc_2 = gst::ElementFactory::make("vaapipostproc", Some("vaapipostproc_2"))?;
    // Initialize vaapijpegenc_2
    let vaapijpegenc_2 = gst::ElementFactory::make("vaapijpegenc", Some("vaapijpegenc_2"))?;
    // Initialize AppSink 2
    let sink_2 = gst::ElementFactory::make("appsink", Some("sink_2"))
        .map_err(|_| MissingElement("appsink"))?;
    // Initialize vaapipostproc_3
    let vaapipostproc_3 = gst::ElementFactory::make("vaapipostproc", Some("vaapipostproc_3"))
        .map_err(|_| MissingElement("vaapipostproc"))?;
    // Initialize vaapijpegenc_3
    let vaapijpegenc_3 = gst::ElementFactory::make("vaapijpegenc", Some("vaapijpegenc_3"))
        .map_err(|_| MissingElement("vaapijpegenc"))?;
    // Initialize AppSink 3
    let sink_3 = gst::ElementFactory::make("appsink", Some("sink_3"))
        .map_err(|_| MissingElement("appsink"))?;
    
    // queue.set_property_from_str("leaky", "downstream");
    // queue_2.set_property_from_str("leaky", "downstream");
    // queue_3.set_property_from_str("leaky", "downstream");
    queue_4.set_property_from_str("leaky", "downstream");
    queue_5.set_property_from_str("leaky", "downstream");
    queue_6.set_property_from_str("leaky", "downstream");
    capsfilter.set_property("caps", &caps);
    capsfilter_2.set_property("caps", &caps_2);
    capsfilter_3.set_property("caps", &caps_3);
    capsfilter_4.set_property("caps", &caps_4);
    capsfilter_5.set_property("caps", &caps_5);
    capsfilter_6.set_property("caps", &caps_6);
    // ADD MANY ELEMENTS TO PIPELINE AND LINK THEM TOGETHER
    let elements = &[
        &src,
        &rtph264depay,
        // &queue,
        &h264parse,
        // &queue_2,
        &vaapih264dec,
        // &queue_3,
        &tee,
        &queue_4,
        &videorate,
        &capsfilter,
        &vaapipostproc,
        &capsfilter_2,
        &vaapijpegenc,
        &sink,
        &queue_5,
        &videorate_2,
        &capsfilter_3,
        &vaapipostproc_2,
        &capsfilter_4,
        &vaapijpegenc_2,
        &sink_2,
        // &queue_6,
        // &videorate_3,
        // &capsfilter_5,
        // &vaapipostproc_3,
        // &capsfilter_6,
        // &vaapijpegenc_3,
        // &sink_3,
    ];

    pipeline.add_many(elements);

    // sink.link(&src)?;
    let _ = src.link(&rtph264depay);
    let rtph264depay_weak = ObjectExt::downgrade(&rtph264depay);
    src.connect_pad_added(move |elm, src_pad| {
        let rtph264depay = match rtph264depay_weak.upgrade() {
            Some(depay) => depay,
            None => return,
        };
        let sink_pad = rtph264depay
            .static_pad("sink")
            .expect("rtph264depay has no sink pad");
        if sink_pad.is_linked() {
            return;
        }
        match src_pad.link(&sink_pad) {
            Ok(_) => (),
            Err(_) => {
                let name = src_pad.name();
                elm.link_pads(Some(name.as_str()), &rtph264depay, Some("sink"));
            }
        };
    });

    rtph264depay.link(&h264parse).unwrap();
    h264parse.link(&vaapih264dec).unwrap();
    vaapih264dec.link(&tee).unwrap();

     // rtph264depay.link(&queue).unwrap();
    //  src.link(&h264parse).unwrap();
     // queue.link(&h264parse).unwrap();
     // h264parse.link(&queue_2).unwrap();
    //  h264parse.link(&vaapih264dec).unwrap();
     // queue_2.link(&vaapih264dec).unwrap();
     // vaapih264dec.link(&queue_3).unwrap();
    //  vaapih264dec.link(&tee).unwrap();
     // queue_3.link(&tee).unwrap();

    tee.link(&queue_4).unwrap();
    queue_4.link(&videorate).unwrap();
    videorate.link(&capsfilter).unwrap();
    capsfilter.link(&vaapipostproc).unwrap();
    vaapipostproc.link(&capsfilter_2).unwrap();
    capsfilter_2.link(&vaapijpegenc).unwrap();
    vaapijpegenc.link(&sink).unwrap();

    tee.link(&queue_5).unwrap();
    queue_5.link(&videorate_2).unwrap();
    videorate_2.link(&capsfilter_3).unwrap();
    capsfilter_3.link(&vaapipostproc_2).unwrap();
    vaapipostproc_2.link(&capsfilter_4).unwrap();
    capsfilter_4.link(&vaapijpegenc_2).unwrap();
    vaapijpegenc_2.link(&sink_2).unwrap();

    // tee.link(&queue_6).unwrap();
    // queue_6.link(&videorate_3).unwrap();
    // videorate_3.link(&capsfilter_5).unwrap();
    // capsfilter_5.link(&vaapipostproc_3).unwrap();
    // vaapipostproc_3.link(&capsfilter_6).unwrap();
    // capsfilter_6.link(&vaapijpegenc_3).unwrap();
    // vaapijpegenc_3.link(&sink_3).unwrap();

    let appsink = pipeline
        .by_name("sink")
        .expect("Sink element not found")
        .downcast::<gst_app::AppSink>()
        .expect("Sink element is expected to be an appsink!");

    let appsink_2 = pipeline
        .by_name("sink_2")
        .expect("Sink 2 element not found")
        .downcast::<gst_app::AppSink>()
        .expect("Sink 2 element is expected to be an appsink!");

    // let appsink_3 = pipeline
    //     .by_name("sink_3")
    //     .expect("Sink 3 element not found")
    //     .downcast::<gst_app::AppSink>()
    //     .expect("Sink 3 element is expected to be an appsink!");

    //FULLSCREEN
    appsink.set_property("emit-signals", false);
    appsink.set_property("max-buffers", 5u32);
    appsink.set_property("drop", true);
    appsink.set_property("sync", true);
    appsink_2.set_property("wait-on-eos", false);

    //THUMNAIL
    appsink_2.set_property("emit-signals", false);
    appsink_2.set_property("max-buffers", 5u32);
    appsink_2.set_property("drop", true);
    appsink_2.set_property("sync", true);
    appsink_2.set_property("wait-on-eos", false);

    //RECORD
    // appsink_3.set_property("emit-signals", false);
    // appsink_3.set_property("max-buffers", 5u32);
    // appsink_3.set_property("drop", true); 
    // appsink_3.set_property("wait-on-eos", false);

    // src.set_property("is-live", true);
    src.set_property("location", &uri);

    let mut count_full = 0;
    let mut count_thumb= 0;
    let mut count_record = 0;
    let id_2 = id.clone();
    let id_3 = id.clone();
    // Getting data out of the appsink is done by setting callbacks on it.
    // The appsink will then call those handlers, as soon as data is available.
    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            // Add a handler to the "new-sample" signal.
            .new_sample(move |appsink| {
                // Pull the sample in question out of the appsink's buffer.
                let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
               //println!("Sample: {:?}", sample);
                let buffer = sample.buffer().ok_or_else(|| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to get buffer from appsink")
                    );

                    gst::FlowError::Error
                })?;

        //        println!("Buffer {:?}", buffer);
                

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

                println!("[FULL] Timestamp: {:?} - cam_id: {:?} - size: {:?}", std::time::SystemTime::now(), id, samples.len());

                // task::block_on(async { client.publish(format!("rtsp_{}", id.clone()).as_str(), samples.to_vec()).await });
                // println!("Uri: {:?} - {:?} bytes", uri.clone(), samples.len());
                 //SAVE IMAGE
                 //let mut file = fs::File::create(format!("img-{}.jpg", count)).unwrap();
                 //file.write_all(samples);

            // if id == "22" {
            //     let img_result = 
            //         image::load_from_memory_with_format(samples, ImageFormat::Jpeg);
            //     match img_result {
            //         Ok(image) => {
            //                //  image.save(format!("full-{}-{}.jpg", id, count_full)).unwrap();
            //                image.save(format!("full-{}-{:?}-{}.jpg", id, std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs(), count_full));
            //                 count_full += 1;
            //            },
            //         Err(_) => (),
            //     };
            // }
            // let mut throttle = Throttle::new(std::time::Duration::from_secs(1), 1);
            // let result = throttle.accept();
            // if result.is_ok() {
                    // println!("Throttle START!!");
                    // let transcode_actor = Distributor::named("transcode");
                    // transcode_actor.tell_one(samples.to_vec()).expect("Tell transcode failed");   
                    
                    drop(samples);
                    drop(map);
                    drop(buffer);
                    drop(sample);
                // }
                Ok(gst::FlowSuccess::Ok)
                // Err(gst::FlowError::Error)
            })
            .build(),
    );

    appsink_2.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            // Add a handler to the "new-sample" signal.
            .new_sample(move |appsink| {
                // Pull the sample in question out of the appsink's buffer.
                let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
               //println!("Sample: {:?}", sample);
                let buffer = sample.buffer().ok_or_else(|| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to get buffer from appsink")
                    );

                    gst::FlowError::Error
                })?;

        //        println!("Buffer {:?}", buffer);
                

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

                println!("[THUMB] Timestamp: {:?} - cam_id: {:?} - size: {:?}", std::time::SystemTime::now(), id_2, samples.len());

                // if id_2 == "22" {
                //     let img_result = 
                //         image::load_from_memory_with_format(samples, ImageFormat::Jpeg);
                //     match img_result {
                //         Ok(image) => {
                //                //  image.save(format!("full-{}-{}.jpg", id, count_full)).unwrap();
                //                image.save(format!("thumb-{}-{:?}.jpg", id_2, std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs()));
                //                 count_full += 1;
                //            },
                //         Err(_) => (),
                //     };
                // }

                // task::block_on(async { client.publish(format!("rtsp_{}", id.clone()).as_str(), samples.to_vec()).await });
                // println!("Uri: {:?} - {:?} bytes", uri.clone(), samples.len());
                 //SAVE IMAGE
                 //let mut file = fs::File::create(format!("img-{}.jpg", count)).unwrap();
                 //file.write_all(samples);

            //  let img_result = 
            //      image::load_from_memory_with_format(samples, ImageFormat::Jpeg);
            //  match img_result {
            //      Ok(image) => {
            //              image.save(format!("thumb-{}-{}.jpg", id_2, count_thumb)).unwrap();
            //              count_thumb += 1;
            //         },
            //      Err(_) => (),
            //  };
            // let mut throttle = Throttle::new(std::time::Duration::from_secs(1), 1);
            // let result = throttle.accept();
            // if result.is_ok() {
                    // println!("Throttle START!!");
                    // let transcode_actor = Distributor::named("transcode");
                    // transcode_actor.tell_one(samples.to_vec()).expect("Tell transcode failed");   
                    
                    drop(samples);
                    drop(map);
                    drop(buffer);
                    drop(sample);
                // }
                Ok(gst::FlowSuccess::Ok)
                // Err(gst::FlowError::Error)
            })
            .build(),
    );

//     appsink_3.set_callbacks(
//         gst_app::AppSinkCallbacks::builder()
//             // Add a handler to the "new-sample" signal.
//             .new_sample(move |appsink| {
//                 // Pull the sample in question out of the appsink's buffer.
//                 let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
//                //println!("Sample: {:?}", sample);
//                 let buffer = sample.buffer().ok_or_else(|| {
//                     element_error!(
//                         appsink,
//                         gst::ResourceError::Failed,
//                         ("Failed to get buffer from appsink")
//                     );

//                     gst::FlowError::Error
//                 })?;

//         //        println!("Buffer {:?}", buffer);
                

//                 let map = buffer.map_readable().map_err(|_| {
//                     element_error!(
//                         appsink,
//                         gst::ResourceError::Failed,
//                         ("Failed to map buffer readable")
//                     );

//                     gst::FlowError::Error
//                 })?;
//   //              println!("xxxxxxxx Map {:?}", map);   

//                 let samples = map.as_slice_of::<u8>().map_err(|_| {
//                     element_error!(
//                         appsink,
//                         gst::ResourceError::Failed,
//                        ("Failed to interprete buffer as S16 PCM")
//                     );

//                     gst::FlowError::Error
//                 })?;

//                 println!("[RECORD] Timestamp: {:?} - cam_id: {:?} - size: {:?}", std::time::SystemTime::now(), id_3, samples.len());

//                 // task::block_on(async { client.publish(format!("rtsp_{}", id.clone()).as_str(), samples.to_vec()).await });
//                 // println!("Uri: {:?} - {:?} bytes", uri.clone(), samples.len());
//                  //SAVE IMAGE
//                  //let mut file = fs::File::create(format!("img-{}.jpg", count)).unwrap();
//                  //file.write_all(samples);

//             if id_3 == "22" {
//                 let img_result = 
//                     image::load_from_memory_with_format(samples, ImageFormat::Jpeg);
//                 match img_result {
//                     Ok(image) => {
//                            //  image.save(format!("full-{}-{}.jpg", id, count_full)).unwrap();
//                            image.save(format!("record-{}-{:?}.jpg", id_3, std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs()));
//                             count_record += 1;
//                        },
//                     Err(_) => (),
//                 };
//             }
//             // let mut throttle = Throttle::new(std::time::Duration::from_secs(1), 1);
//             // let result = throttle.accept();
//             // if result.is_ok() {
//                     // println!("Throttle START!!");
//                     // let transcode_actor = Distributor::named("transcode");
//                     // transcode_actor.tell_one(samples.to_vec()).expect("Tell transcode failed");   
                    
//                     drop(samples);
//                     drop(map);
//                     drop(buffer);
//                     drop(sample);
//                 // }
//                 Ok(gst::FlowSuccess::Ok)
//                 // Err(gst::FlowError::Error)
//             })
//             .build(),
//     );

    Ok(pipeline)
}

fn main_loop(pipeline: gst::Pipeline) -> Result<(), Error> {
    println!("Start main loop");
    pipeline.set_state(gst::State::Playing)?;
    
    let bus = pipeline
    .bus()
    .expect("Pipeline without bus. Shouldn't happen!");
    
    //    println!("Bus: {:?}", bus);
    
    use gst::MessageView;
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        // println!("In loop msg: {:?}", msg);

        match msg.view() {
            MessageView::Eos(..) => {
                break;
            },
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null)?;
                println!("Error: {:?}", err.error());
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
            MessageView::SegmentDone(_) => {
                match pipeline.seek(
                    1.0, 
                    SeekFlags::SEGMENT, 
                    SeekType::Set, 
                    gst::ClockTime::from_seconds(0), 
                    SeekType::None, 
                    gst::ClockTime::from_seconds(0)) {
                    Ok(_) => println!("Ok"),
                    Err(err) => println!("{:?}", err)
                }
            }
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;

    Ok(())
}
#[tokio::main]
async fn main() {
    // let handle = Handle::current();
    

    let urls = [
        "rtsp://10.50.29.96/1/h264major",
        "rtsp://10.50.31.171/1/h264major",
        "rtsp://10.50.13.231/1/h264major",
        "rtsp://10.50.13.233/1/h264major",
        "rtsp://10.50.13.234/1/h264major",
        "rtsp://10.50.13.235/1/h264major",
        "rtsp://10.50.13.236/1/h264major",
        "rtsp://10.50.13.237/1/h264major",
        "rtsp://10.50.13.238/1/h264major",
        "rtsp://10.50.13.239/1/h264major",
        "rtsp://10.50.13.240/1/h264major",
        "rtsp://10.50.13.241/1/h264major",
        "rtsp://10.50.13.242/1/h264major",
        "rtsp://10.50.13.243/1/h264major",
        "rtsp://10.50.13.244/1/h264major",
        "rtsp://10.50.13.245/1/h264major",
        "rtsp://10.50.13.248/1/h264major",
        "rtsp://10.50.13.249/1/h264major",
        "rtsp://10.50.13.252/1/h264major",
        "rtsp://10.50.13.253/1/h264major",
        "rtsp://10.50.13.254/1/h264major",
        // "http://vietnam:L3xRay123!@10.50.29.22/mjpgstreamreq/1/image.jpg",
        // "http://10.50.31.171/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.231/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.233/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.234/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.235/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.236/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.237/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.238/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.239/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.240/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.241/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.242/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.243/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.244/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.245/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.248/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.249/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.252/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.252/mjpgstreamreq/1/image.jpg",
        // "http://10.50.13.254/mjpgstreamreq/1/image.jpg",
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
        96, 
        171,
        231, 
        233, 
        234, 
        235, 
        236, 
        237, 
        238, 
        239, 
        240,
        241,
        242, 
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
    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut index = 0;
    // let client = task::block_on(connect_nats());
    for ip in &cam_ip {
        let name = format!("rtsp-{}", ip);
        let rtsp_actor = Distributor::named(name);
        let msg = RTPMessage {
            url: urls[index].to_owned(),
            // client: client.clone(),
            id: ip.to_string(),
        };
        rtsp_actor.tell_one(msg).expect("tell failed");
        index += 1;
    }

Bastion::block_until_stopped();

}

async fn get_rtsp_stream(ctx: BastionContext) -> Result<(), ()> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    //let mut rng = rand::thread_rng();
    loop {
        MessageHandler::new(ctx.recv().await?)
            .on_tell(|message: RTPMessage, _| {
//let mut rng = rand::thread_rng();                
//let n1: u8 = rng.gen();
//println!("spawn new actor: {:?} - {:?}", message, n1);
                rt.spawn_blocking( move || {  
                  create_pipeline(message.id, message.url).and_then(|pipeline| main_loop(pipeline));
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
