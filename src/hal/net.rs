use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    sync::mpsc::{Receiver, Sender, channel},
};

use super::cameras::Image;
use async_io::Async;
use bevy::{prelude::*, tasks::IoTaskPool};
use bytemuck::{NoUninit, bytes_of};
use futures_lite::{AsyncReadExt, AsyncWriteExt, StreamExt as _, pin};

#[derive(Debug, Default, Clone)]
pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<IncomingMessage>()
            .add_systems(PreUpdate, receiver);
    }
}

fn receiver(
    mut incoming: EventWriter<IncomingMessage>,
    mut receiver: Local<Option<Receiver<IncomingMessage>>>,
) -> Result {
    once!(*receiver = Some(server()?));
    let receiver = receiver.as_mut().unwrap();
    while let Ok(message) = receiver.try_recv() {
        incoming.write(message);
    }
    Ok(())
}

pub const SIM_IP: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1818);
pub const HAL_IP: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1817);

fn server() -> Result<Receiver<IncomingMessage>> {
    let (tx, rx) = channel();
    let listener = Async::<TcpListener>::bind(SIM_IP)?;
    IoTaskPool::get()
        .spawn(async move {
            let incoming = listener.incoming();
            pin!(incoming);

            while let Some(stream) = incoming.next().await {
                let Ok(stream) = stream else {
                    continue;
                };
                let tx = tx.clone();
                IoTaskPool::get()
                    .spawn(async move { handle_connection(stream, tx) })
                    .detach();
            }
        })
        .detach();
    Ok(rx)
}

async fn handle_connection(mut stream: Async<TcpStream>, tx: Sender<IncomingMessage>) -> Result {
    let mut len = [0u8; 8];
    stream.read_exact(&mut len).await?;
    let _len = u64::from_be_bytes(len);
    let mut kind = 0u8;
    stream.read_exact(std::array::from_mut(&mut kind)).await?;
    let kind = MessageKind::try_from(kind)?;
    let message = match kind {
        MessageKind::Motors => {
            let mut data = [[0u8; 4]; 8];
            stream.read_exact(data.as_flattened_mut()).await?;
            IncomingMessage::Motors(data.map(f32::from_be_bytes))
        }
        MessageKind::BotcamOn => {
            let mut data = 0u8;
            stream.read_exact(std::array::from_mut(&mut data)).await?;
            IncomingMessage::BotcamOn(data != 0)
        }
        MessageKind::ZedOn => {
            let mut data = 0u8;
            stream.read_exact(std::array::from_mut(&mut data)).await?;
            IncomingMessage::ZedOn(data != 0)
        }
        _ => {
            return Err("Should not receive incoming sensors or images".into());
        }
    };
    let _ = tx.send(message);
    Ok(())
}

#[repr(u8)]
pub enum MessageKind {
    Sensors = 1,
    BotcamImage = 2,
    ZedImage = 3,
    Motors = 4,
    BotcamOn = 5,
    ZedOn = 6,
}

impl TryFrom<u8> for MessageKind {
    type Error = BevyError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::Sensors),
            2 => Ok(Self::BotcamImage),
            3 => Ok(Self::ZedImage),
            4 => Ok(Self::Motors),
            5 => Ok(Self::BotcamOn),
            6 => Ok(Self::ZedOn),
            _ => Err("Invalid message kind".into()),
        }
    }
}

#[derive(Debug, Clone, Copy, NoUninit)]
#[repr(C)]
pub struct Dvl {
    velocity_a: f32,
    velocity_b: f32,
    velocity_c: f32,
}

#[derive(Debug, Clone, Copy, NoUninit)]
#[repr(C)]
pub struct ImuINS {
    theta: [f32; 3],
}

#[derive(Debug, Clone, Copy, NoUninit)]
#[repr(C)]
pub struct ImuPIMU {
    dtheta: [f32; 3],
    dvel: [f32; 3],
    dt: f32,
}

#[derive(Debug, Clone, Copy, NoUninit)]
#[repr(C)]
pub struct SensorMessage {
    depth: f32,
    dvl: Dvl,
    imu_ins: ImuINS,
    imu_pimu: ImuPIMU,
}

pub enum OutgoingMessage {
    Sensors(SensorMessage),
    BotcamImage(Image),
    ZedImage(Image),
}

impl OutgoingMessage {
    fn kind(&self) -> MessageKind {
        self.into()
    }

    fn len(&self) -> u64 {
        (1 + match self {
            OutgoingMessage::Sensors(sensors) => std::mem::size_of_val(sensors),
            OutgoingMessage::BotcamImage(image) | OutgoingMessage::ZedImage(image) => {
                std::mem::size_of::<u32>() * 2 + image.buffer.len()
            }
        }) as u64
    }
}

impl Into<MessageKind> for &OutgoingMessage {
    fn into(self) -> MessageKind {
        match self {
            OutgoingMessage::Sensors(_) => MessageKind::Sensors,
            OutgoingMessage::BotcamImage(_) => MessageKind::BotcamImage,
            OutgoingMessage::ZedImage(_) => MessageKind::ZedImage,
        }
    }
}

#[derive(Debug, Event)]
pub enum IncomingMessage {
    Motors([f32; 8]),
    BotcamOn(bool),
    ZedOn(bool),
}

pub async fn send(message: OutgoingMessage) -> Result {
    let mut client = Async::<TcpStream>::connect(HAL_IP).await?;
    client.write_all(&message.len().to_be_bytes()).await?;
    client.write_all(&[message.kind() as u8]).await?;
    match message {
        OutgoingMessage::Sensors(sensors) => {
            client.write_all(bytes_of(&sensors)).await?;
        }
        OutgoingMessage::BotcamImage(image) | OutgoingMessage::ZedImage(image) => {
            client.write_all(&image.width.to_be_bytes()).await?;
            client.write_all(&image.height.to_be_bytes()).await?;
            client.write_all(&image.buffer).await?;
        }
    }
    client.flush().await?;
    Ok(())
}
