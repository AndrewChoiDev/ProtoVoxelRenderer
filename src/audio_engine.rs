use cpal::traits::{DeviceTrait, HostTrait, EventLoopTrait};
use cpal::{StreamData, UnknownTypeOutputBuffer};
use cpal::OutputBuffer;

use hound;


pub fn run()
{
    let host = cpal::default_host();
    let event_loop = host.event_loop();
    let device = 
        host.default_output_device()
        .expect("Cpal Error: Could not find a default output device");
    let mut supported_formats_range = 
        device.supported_output_formats()
        .expect("Cpal Error: Could not query formats");
    let format = 
        supported_formats_range.next()
        .expect("Cpal Error: Could not find a supported format")
        .with_max_sample_rate();
    
    let stream_id = event_loop.build_output_stream(&device, &format).unwrap();


    let a = hound::WavReader::open("audio/test.wav").unwrap().into_samples::<i16>();
    let mut reader : Vec<f32> = 
        a.map(|b| cpal::Sample::to_f32(&b.unwrap())).collect::<Vec<f32>>().into_iter().rev().collect();
    println!("{}", reader.len());


    event_loop.play_stream(stream_id).expect("Cpal Error: Could not play stream");
    event_loop.run(move |stream_id, stream_result| {
        let stream_data = match stream_result {
            Ok(data) => data,
            Err(err) => 
            {
                eprintln!("Cpal Error: occured on stream {:?} - {}", stream_id, err);
                return;
            }
        };

        match stream_data {
            StreamData::Output {buffer} => 
            {
                match buffer
                {
                    UnknownTypeOutputBuffer::I16(buffer) =>
                        write_to_output_buffer::<i16>(buffer, &mut reader),
                    UnknownTypeOutputBuffer::U16(buffer) =>
                        write_to_output_buffer::<u16>(buffer, &mut reader),
                    UnknownTypeOutputBuffer::F32(buffer) =>
                        write_to_output_buffer::<f32>(buffer, &mut reader),
                }
            }
            _ => ()
        }

    });
}
// use cpal::SampleFormat::
fn write_to_output_buffer<T>(mut buffer : OutputBuffer<'_, T>, reader : &mut Vec<f32>)
    where T : cpal::Sample
{

    for elem in buffer.iter_mut()
    {
        reader.pop();
        let value : f32 = reader.pop().unwrap_or(0.0) * rand::random::<f32>();
        // println!("{}", value);
        let sample_value : T = cpal::Sample::from(&value);
        *elem = sample_value;
    }
}