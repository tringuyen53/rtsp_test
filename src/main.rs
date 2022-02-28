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

 fn create_pipeline(id: String, uri: String, client: Connection) -> Result<gst::Pipeline, Error> {
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
        //MJPEG
    let pipeline = gst::parse_launch(&format!(
        "souphttpsrc location={} ! jpegparse ! vaapijpegdec !
        videorate ! video/x-raw, framerate=2/1 ! vaapijpegenc ! appsink name=app1 emit-signals=false drop=true sync=false" ,
        uri
    ))?
    .downcast::<gst::Pipeline>()
    .expect("Expected a gst::Pipeline");

    println!("pipeline: {:?} - {:?}", uri, pipeline);
    
    // Get access to the appsink element.
    let appsink1 = pipeline
        .by_name("app1")
        .expect("Sink element not found")
        .downcast::<gst_app::AppSink>()
        .expect("Sink element is expected to be an appsink!");

    let appsink2 = pipeline
        .by_name("app2")
        .expect("Sink element not found")
        .downcast::<gst_app::AppSink>()
        .expect("Sink element is expected to be an appsink!");

    let mut count_full = 0;
    let mut count_thumb= 0;
    let id_2 = id.clone();
    // Getting data out of the appsink is done by setting callbacks on it.
    // The appsink will then call those handlers, as soon as data is available.
    appsink1.set_callbacks(
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

            //  let img_result = 
            //      image::load_from_memory_with_format(samples, ImageFormat::Jpeg);
            //  match img_result {
            //      Ok(image) => {
            //              image.save(format!("full-{}-{}.jpg", id, count_full)).unwrap();
            //              count_full += 1;
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

    appsink2.set_callbacks(
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

    Ok(pipeline)
}

fn main_loop(pipeline: gst::Pipeline) -> Result<(), Error> {
    println!("Start main loop");
    pipeline.set_state(gst::State::Playing)?;

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

//    println!("Bus: {:?}", bus);

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        // println!("In loop msg: {:?}", msg);
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
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
        // "rtsp://10.50.29.36/1/h264major",
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
        // "rtsp://10.50.13.252/1/h264major",
        // "rtsp://10.50.13.253/1/h264major",
        // "rtsp://10.50.13.254/1/h264major",
        // "http://10.50.29.36/mjpgstreamreq/1/image.jpg",
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
        "http://10.50.13.254/mjpgstreamreq/1/image.jpg",
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
        // 240,
        // 241,
        // 242, 
        // 243, 
        // 244, 
        // 245, 
        // 248, 
        // 249, 
        // 252, 
        // 253, 
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

    // let rtsp_actor = Distributor::named("rtsp");
    // let rtsp1_actor = Distributor::named("rtsp-1");
    // let rtsp2_actor = Distributor::named("rtsp-2");
    // let rtsp3_actor = Distributor::named("rtsp-3");
    // let rtsp4_actor = Distributor::named("rtsp-4");
    // let rtsp5_actor = Distributor::named("rtsp-5");
    // let rtsp6_actor = Distributor::named("rtsp-6");
    // let rtsp7_actor = Distributor::named("rtsp-7");
    // let rtsp8_actor = Distributor::named("rtsp-8");
    // let rtsp9_actor = Distributor::named("rtsp-9");
    // let rtsp10_actor = Distributor::named("rtsp-10");
    // let rtsp11_actor = Distributor::named("rtsp-11");
    // let rtsp12_actor = Distributor::named("rtsp-12");
    // let rtsp13_actor = Distributor::named("rtsp-13");
    // let rtsp14_actor = Distributor::named("rtsp-14");
    // let rtsp15_actor = Distributor::named("rtsp-15");
//    for url in urls {
//        rtsp_actor.tell_one(url).expect("tell failed");
//    }
// rtsp_actor.tell_one("rtsp://10.50.13.231/1/h264major").expect("tell failed");
// rtsp1_actor.tell_one("rtsp://10.50.13.233/1/h264major").expect("tell failed");
// rtsp2_actor.tell_one("rtsp://10.50.13.234/1/h264major").expect("tell failed");
// rtsp3_actor.tell_one("rtsp://10.50.13.235/1/h264major").expect("tell failed");
// rtsp4_actor.tell_one("rtsp://10.50.13.236/1/h264major").expect("tell failed");
// rtsp5_actor.tell_one("rtsp://10.50.13.237/1/h264major").expect("tell failed");
// rtsp6_actor.tell_one("rtsp://10.50.13.238/1/h264major").expect("tell failed");
// rtsp7_actor.tell_one("rtsp://10.50.13.239/1/h264major").expect("tell failed");
// rtsp8_actor.tell_one("rtsp://10.50.13.240/1/h264major").expect("tell failed");
// rtsp9_actor.tell_one("rtsp://10.50.13.241/1/h264major").expect("tell failed");
// rtsp10_actor.tell_one("rtsp://10.50.13.242/1/h264major").expect("tell failed");
// rtsp11_actor.tell_one("rtsp://10.50.13.243/1/h264major").expect("tell failed");
// rtsp12_actor.tell_one("rtsp://10.50.13.244/1/h264major").expect("tell failed");
// rtsp13_actor.tell_one("rtsp://10.50.13.245/1/h264major").expect("tell failed");
// rtsp14_actor.tell_one("rtsp://10.50.13.248/1/h264major").expect("tell failed");
// rtsp15_actor.tell_one("rtsp://10.50.13.249/1/h264major").expect("tell failed");
//rtsp_actor.tell_one("rtsp://10.50.13.248/1/h264major").expect("tell failed");
//rtsp_actor.tell_one("rtsp://10.50.13.249/1/h264major").expect("tell failed");
//rtsp_actor.tell_one("rtsp://10.50.13.250/1/h264major").expect("tell failed");
//rtsp_actor.tell_one("rtsp://10.50.13.251/1/h264major").expect("tell failed");
// rtsp_actor.tell_one("rtsp://10.50.13.252/1/h264major").expect("tell failed");
//rtsp2_actor.tell_one("rtsp://10.50.13.254/1/h264major").expect("tell failed");
//rtsp_actor.tell_one("rtsp://vietnam:L3xRay123!@10.50.12.187/media/video1").expect("tell failed");
//println!("Result: {:?}", res);    
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
                  create_pipeline(message.id, message.url, message.client).and_then(|pipeline| main_loop(pipeline));
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
