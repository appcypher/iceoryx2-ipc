// Request - Reply

use std::time::Duration;

use iceoryx2::prelude::*;
use iceoryx2_bb_container::{byte_string::FixedSizeByteString, vec::FixedSizeVec};

fn main() -> anyhow::Result<()> {
    // Read the first argument
    let arg = match std::env::args().nth(1) {
        Some(arg) => arg,
        None => {
            eprintln!("Usage: test_pubsub <arg>");
            std::process::exit(1);
        }
    };

    if arg == "client" {
        client()?;
    } else if arg == "server" {
        server()?;
    }

    Ok(())
}

const LONG_CYCLE_TIME: Duration = Duration::from_secs(10);
const SHORT_CYCLE_TIME: Duration = Duration::from_secs(1);
const TEXT_CAPACITY: usize = 512;
const PAYLOAD_CAPACITY: usize = 64 * 1024;

#[repr(C)]
#[derive(Debug, Default)]
pub enum Tag {
    #[default]
    Bytes = 0,
    Cbor = 1,
}

#[repr(C)]
#[derive(Debug, Default, PlacementDefault)]
pub struct Request {
    pub tag: u64,
    pub reply_channel: Option<FixedSizeByteString<TEXT_CAPACITY>>,
    pub payload: FixedSizeVec<u8, PAYLOAD_CAPACITY>,
}

fn client() -> anyhow::Result<()> {
    println!("Client");

    let node = NodeBuilder::new().create::<zero_copy::Service>()?;
    let request_service = node
        .service_builder("test/request".try_into()?)
        .publish_subscribe::<Request>()
        .max_publishers(1)
        .max_subscribers(1)
        .open_or_create()?;

    let publisher = request_service.publisher_builder().create()?;

    while let NodeEvent::Tick = node.wait(LONG_CYCLE_TIME) {
        // Send request.

        let sample = publisher.loan_uninit()?;
        let sample = sample.write_payload(Request {
            tag: Tag::Cbor as u64,
            reply_channel: Some(b"test/reply".into()),
            payload: FixedSizeVec::new(),
        });
        sample.send()?;
        println!("Request Sent");

        // Now wait for response.

        let reply_service = node
            .service_builder("test/reply".try_into()?)
            .publish_subscribe::<Request>()
            .max_publishers(1)
            .max_subscribers(1)
            .open_or_create()?;

        // Recieve response.

        let subscriber = reply_service.subscriber_builder().create()?;
        while let NodeEvent::Tick = node.wait(SHORT_CYCLE_TIME) {
            while let Some(sample) = subscriber.receive()? {
                println!("Response Recieved: {:?}", sample.reply_channel);
            }
        }
    }

    Ok(())
}

fn server() -> anyhow::Result<()> {
    println!("Server");

    let node = NodeBuilder::new().create::<zero_copy::Service>()?;
    let request_service = node
        .service_builder("test/request".try_into()?)
        .publish_subscribe::<Request>()
        .max_publishers(1)
        .max_subscribers(1)
        .open_or_create()?;

    let subscriber = request_service.subscriber_builder().create()?;

    while let NodeEvent::Tick = node.wait(SHORT_CYCLE_TIME) {
        while let Some(sample) = subscriber.receive()? {
            // Recieve request.

            println!("Request Recieved: {:?}", sample.reply_channel);

            // Now send response.

            let reply_channel = sample.reply_channel.unwrap().to_string();
            let reply_service = node
                .service_builder(reply_channel.as_str().try_into()?)
                .publish_subscribe::<Request>()
                .max_publishers(1)
                .max_subscribers(1)
                .open_or_create()?;

            let publisher = reply_service.publisher_builder().create()?;

            let sample = publisher.loan_uninit()?;
            let sample = sample.write_payload(Request {
                tag: Tag::Cbor as u64,
                reply_channel: None,
                payload: FixedSizeVec::new(),
            });
            sample.send()?;
            println!("Response Sent");
        }
    }

    Ok(())
}
