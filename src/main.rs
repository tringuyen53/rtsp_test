use opencv::{Result, imgcodecs, prelude::*, videoio};
use tokio::runtime::Handle;
// use opencv::prelude::*;
#[tokio::main]
async fn main() {
    
	let handle = Handle::current();

    let urls = [
        "rtsp://vietnam:L3xRay123!@10.50.30.212/1/h264major",
        "rtsp://10.50.29.36/1/h264major",
        "rtsp://vietnam:L3xRay123!@10.50.29.64/axis-media/media.amp",
        "rtsp://vietnam:L3xRay123!@10.50.12.187/media/video1",
        "rtsp://10.50.30.100/1/h264major",

    ];

    for url in urls {
        handle.spawn(async move { get_frame(url).await });
    }

    loop {}
}

async fn get_frame(cam_url: &str) -> Result<(), opencv::Error> {
    println!("{:?}", cam_url);
    let mut cam = videoio::VideoCapture::from_file(cam_url,
     videoio::CAP_FFMPEG).unwrap(); // 0 is the default camera
    let opened = videoio::VideoCapture::is_opened(&cam).unwrap();
	if !opened {
		panic!("Unable to open default camera!");
	}
    let mut img_count = 0;
    let mut count = 0;
    let mut params = opencv::types::VectorOfi32::new();
    let mut frame_buffer = opencv::types::VectorOfu8::new();
    params.push(imgcodecs::IMWRITE_JPEG_QUALITY);
    params.push(50);
    loop {
		let mut frame = Mat::default();
		let is_read = cam.read(&mut frame)?;
        if is_read {
            //Save frame to jpg image.
                // imgcodecs::imwrite(format!("img-{}.jpg",img_count).as_str(), &frame, &params)?;
                imgcodecs::imencode(".jpg", &frame, &mut frame_buffer, &params)?;
                println!("frame buffer length: {}", frame_buffer.len());
            //     let img_result =
            //     image::load_from_memory_with_format(frame_buffer.as_slice(), image::ImageFormat::Jpeg);
            // let img = match img_result {
            //     Ok(image) => image,
            //     Err(_) => {
            //         println!("not image format");
            //         return Ok(())
            //     },
            // };
            // img.save(format!("{:?}-{}.jpg", img.clone().as_bytes().len(), img_count)).unwrap();
            // let img16 = img.into_rgb8();
            // let data = img16.into_raw() as Vec<u8>;
            // println!("Image length: {}", data.len());
            count += 1;
            count += 15;
            let val = cam.get(videoio::CAP_PROP_POS_FRAMES)?;
            println!("Get: {}", val);
            cam.set(videoio::CAP_PROP_POS_FRAMES, count as f64)?;
        } else {
            cam.release();
            break;
        }
        println!("End of 1 frame.");
        img_count += 1;
		// let key = highgui::wait_key(10)?;
		// if key > 0 && key != 255 {
		// 	break;
		// }
	}
    Ok(())
}
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
//         // "rtsp://vietnam:L3xRay123!@10.50.29.64/axis-media/media.amp",
//         // "rtsp://vietnam:L3xRay123!@10.50.12.187/media/video1",
//         // "rtsp://10.50.30.100/1/h264major",

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
