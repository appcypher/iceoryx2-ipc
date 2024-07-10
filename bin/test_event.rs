use std::time::Duration;

use iceoryx2::prelude::*;

fn main() -> anyhow::Result<()> {
    // Read the first argument
    let arg = match std::env::args().nth(1) {
        Some(arg) => arg,
        None => {
            eprintln!("Usage: test_event <arg>");
            std::process::exit(1);
        }
    };

    if arg == "listener" {
        listener()?;
    } else if arg == "notifier" {
        notifier()?;
    }

    Ok(())
}

const CYCLE_TIME: Duration = Duration::from_secs(1);

fn listener() -> anyhow::Result<()> {
    println!("Listener");

    let node = NodeBuilder::new().create::<zero_copy::Service>()?;
    let service = node.service_builder("test/path".try_into()?)
        .event()
        .max_notifiers(1)
        .max_listeners(1)
        .open_or_create()?;

    let listener = service.listener_builder().create()?;

    while let NodeEvent::Tick = node.wait(Duration::ZERO) {
        if let Ok(Some(event_id)) = listener.timed_wait_one(CYCLE_TIME) {
            println!("Received: {:?}", event_id);
        }
    }

    Ok(())
}

fn notifier() -> anyhow::Result<()> {
    println!("Notifier");

    let node = NodeBuilder::new().create::<zero_copy::Service>()?;
    let service = node.service_builder("test/path".try_into()?)
        .event()
        .max_notifiers(1)
        .max_listeners(1)
        .open_or_create()?;

    let notifier = service.notifier_builder().create()?;

    let mut counter: usize = 0;

    while let NodeEvent::Tick = node.wait(CYCLE_TIME) {
        counter += 1;
        notifier.notify_with_custom_event_id(EventId::new(counter))?;

        println!("Notified: {:?}", counter);
    }

    Ok(())
}
