use std::{pin::Pin, time::Duration};

use anyhow::Result;
use iceoryx2::{
    node::{Node, NodeBuilder, NodeEvent},
    prelude::PlacementDefault,
    service::{port_factory::publish_subscribe::PortFactory, zero_copy},
};
use iceoryx2_bb_container::{queue::FixedSizeQueue, vec::FixedSizeVec};
use tokio::io::{AsyncRead, AsyncReadExt};

//-------------------------------------------------------------------------------------------------
// Constants
//-------------------------------------------------------------------------------------------------

const CYCLE_TIME: Duration = Duration::from_millis(10);
const QUEUE_CAPACITY: usize = 1024;
const CHUNK_CAPACITY: usize = 1024;

//-------------------------------------------------------------------------------------------------
// Types
//-------------------------------------------------------------------------------------------------

pub type Chunk = FixedSizeVec<u8, CHUNK_CAPACITY>;

pub type Ack = i64;

#[repr(C)]
#[derive(Debug, Default, PlacementDefault)]
pub struct Request {
    pub codec: u64,
    pub payload_queue: FixedSizeQueue<Chunk, QUEUE_CAPACITY>,
}

pub struct Listener {
    node: Node<zero_copy::Service>,
    request_service: PortFactory<zero_copy::Service, Request, ()>,
    request_ack_service: PortFactory<zero_copy::Service, Ack, ()>,
}

pub struct Client {
    node: Node<zero_copy::Service>,
    request_service: PortFactory<zero_copy::Service, Request, ()>,
    request_ack_service: PortFactory<zero_copy::Service, Ack, ()>,
    bytes_reader: Option<Pin<Box<dyn AsyncRead + Send>>>, // TODO: Need a builder pattern here
}

pub struct Response {}

pub struct PayloadReader {}

//-------------------------------------------------------------------------------------------------
// Methods
//-------------------------------------------------------------------------------------------------

impl Listener {
    pub fn bind(path: &str) -> Result<Self> {
        let node = NodeBuilder::new().create::<zero_copy::Service>()?;

        let request_service = node
            .service_builder(&path.try_into()?)
            .publish_subscribe::<Request>()
            .max_publishers(1)
            .max_subscribers(1)
            .open_or_create()?;

        let request_ack_service = node
            .service_builder(&format!("{}-ack", path).as_str().try_into()?)
            .publish_subscribe::<Ack>()
            .max_publishers(1)
            .max_subscribers(1)
            .open_or_create()?;

        Ok(Self {
            node,
            request_service,
            request_ack_service,
        })
    }
}

// Client::connect("") [-> Client] .payload(P).send() [-> Response] .payload()
impl Client {
    pub fn connect(path: &str) -> Result<Self> {
        let node = NodeBuilder::new().create::<zero_copy::Service>()?;

        let request_service = node
            .service_builder(&path.try_into()?)
            .publish_subscribe::<Request>()
            .max_publishers(1)
            .max_subscribers(1)
            .open_or_create()?;

        let request_ack_service = node
            .service_builder(&format!("{}-ack", path).as_str().try_into()?)
            .publish_subscribe::<Ack>()
            .max_publishers(1)
            .max_subscribers(1)
            .open_or_create()?;

        Ok(Self {
            node,
            request_service,
            request_ack_service,
            bytes_reader: None,
        })
    }

    pub fn bytes(self, payload: impl AsyncRead + Send + 'static) -> Self {
        Self {
            bytes_reader: Some(Box::pin(payload)),
            ..self
        }
    }

    pub async fn send(self) -> Result<Response> {
        let publisher = self.request_service.publisher_builder().create()?;
        let subscriber = self.request_ack_service.subscriber_builder().create()?;

        // Set up the necessary indices.
        let mut push_index = 0;
        let mut ack_index = 0;

        // Get the payload reader
        let mut payload_reader = self
            .bytes_reader
            .ok_or_else(|| anyhow::anyhow!("Payload reader not set"))?;

        // Initialize the sample
        let sample = publisher.loan_uninit()?;
        let mut sample = sample.write_payload(Request {
            codec: 0,
            payload_queue: FixedSizeQueue::new(),
        });

        let s = publisher.loan()?;

        // Event loop.
        while let NodeEvent::Tick = self.node.wait(CYCLE_TIME) {
            // --  PUSH UPDATE
            if push_index - ack_index < QUEUE_CAPACITY {
                // Read from the payload reader and push to the queue.
                let mut chunk = [0u8; CHUNK_CAPACITY];
                let n = payload_reader.read(&mut chunk).await?;
                if n == 0 {
                    break;
                }

                // // Push the chunk to the queue.
                // sample
                //     .payload_mut()
                //     .payload_queue
                //     .push(chunk_from_slice(&chunk, n)?);

                // // Increment the push index.
                // push_index += 1;

                // // Publish the sample.
                // sample.send();
            }

            // -- ACK UPDATE
        }

        todo!()
    }
}

impl Response {}

impl PayloadReader {}

//-------------------------------------------------------------------------------------------------
// Methods
//-------------------------------------------------------------------------------------------------

pub async fn serve() -> Result<()> {
    todo!()
}

//-------------------------------------------------------------------------------------------------
// Methods: Utils
//-------------------------------------------------------------------------------------------------

fn chunk_from_slice(slice: &[u8], len: usize) -> Result<Chunk> {
    let mut chunk = Chunk::new();
    chunk.clone_from_slice(&slice[..len]);
    Ok(chunk)
}
