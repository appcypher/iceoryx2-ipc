use std::time::Duration;

use iceoryx2::prelude::*;

fn main() -> anyhow::Result<()> {
    // Read the first argument
    let arg = match std::env::args().nth(1) {
        Some(arg) => arg,
        None => {
            eprintln!("Usage: test_pubsub <arg>");
            std::process::exit(1);
        }
    };

    if arg == "sub" {
        subscriber()?;
    } else if arg == "pub" {
        publisher()?;
    }

    Ok(())
}

const CYCLE_TIME: Duration = Duration::from_secs(1);

fn subscriber() -> anyhow::Result<()> {
    println!("Subscriber");

    let node = NodeBuilder::new().create::<zero_copy::Service>()?;
    let service = node.service_builder("test/path".try_into()?)
        .publish_subscribe::<u64>()
        .max_publishers(1)
        .max_subscribers(1)
        .open_or_create()?;

    let subscriber = service.subscriber_builder().create()?;

    while let NodeEvent::Tick = node.wait(CYCLE_TIME) {
        while let Some(sample) = subscriber.receive()? {
            println!("Received: {:?}", sample);
        }
    }

    Ok(())
}

fn publisher() -> anyhow::Result<()> {
    println!("Publisher");

    let node = NodeBuilder::new().create::<zero_copy::Service>()?;
    let service = node.service_builder("test/path".try_into()?)
        .publish_subscribe::<u64>()
        .max_publishers(1)
        .max_subscribers(1)
        .open_or_create()?;

    let publisher = service.publisher_builder().create()?;

    while let NodeEvent::Tick = node.wait(CYCLE_TIME) {
        let sample = publisher.loan_uninit()?;
        let sample = sample.write_payload(1234);
        sample.send()?;
    }

    Ok(())
}
