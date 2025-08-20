use std::{
    mem::forget,
    mem::{offset_of, size_of},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{Receiver, channel},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::utils::flatten_array;

use super::cameras::Image;
use async_io::Async;
use bevy::{prelude::*, tasks::IoTaskPool};
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use smallvec::SmallVec;

#[derive(Debug, Default, Clone)]
pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<IncomingMessage>()
            .add_systems(PreUpdate, receiver)
            .add_systems(Update, dbg_send_count);
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

static MESSAGES_STARTED: AtomicUsize = AtomicUsize::new(0);
static MESSAGES_FINISHED: AtomicUsize = AtomicUsize::new(0);
static MESSAGES_CANCELLED: AtomicUsize = AtomicUsize::new(0);

fn dbg_send_count(mut count: Local<usize>) {
    *count += 1;
    if *count != 100 {
        return;
    }
    let ticks = *count;
    *count = 0;
    // let started = MESSAGES_STARTED.swap(0, Ordering::Relaxed);
    // let finished = MESSAGES_FINISHED.swap(0, Ordering::Relaxed);
    // if started == finished {
    //     return;
    // }
    // warn!(
    //     "started: {}; finished: {}; ratio: {}; in {} ticks",
    //     started,
    //     finished,
    //     finished as f64 / started as f64,
    //     ticks
    // );
    let cancelled = MESSAGES_CANCELLED.swap(0, Ordering::Relaxed);
    if cancelled != 0 {
        warn!(
            "{} messages cancelled in the last {} ticks",
            cancelled, ticks
        )
    }
}

pub const HAL_INCOMING: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1817);
pub const HAL_OUTGOING: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1818);

fn server() -> Result<Receiver<IncomingMessage>> {
    let (tx, rx) = channel();
    IoTaskPool::get()
        .spawn(async move {
            loop {
                let Ok(mut client) = Async::<TcpStream>::connect(HAL_OUTGOING).await else {
                    continue;
                };
                info!("Connection to HAL established");
                loop {
                    match handle_connection(&mut client).await {
                        Ok(Some(message)) => {
                            tx.send(message).expect("Connection should not have closed");
                        }
                        Ok(None) => {
                            const WAIT_PERIOD: Duration = Duration::from_millis(1000);
                            async_io::Timer::after(WAIT_PERIOD).await;
                            break;
                        }
                        Err(e) => {
                            warn!("Failed to read incoming data from HAL: {}", e);
                            break;
                        }
                    }
                }
            }
        })
        .detach();
    Ok(rx)
}

async fn handle_connection(stream: &mut Async<TcpStream>) -> Result<Option<IncomingMessage>> {
    let mut len = [0u8; 8];
    match stream.read_exact(&mut len).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
    }
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
        MessageKind::LocalizationEstimate => {
            let mut data = [[0u8; 4]; 15];
            stream.read_exact(data.as_flattened_mut()).await?;
            let data = data.map(f32::from_be_bytes);
            IncomingMessage::LocalizationEstimate {
                rotation: Mat3::from_cols_slice(&data[..9]),
                position: Vec3::from_slice(&data[9..12]),
                velocity: Vec3::from_slice(&data[12..15]),
            }
        }
        _ => {
            return Err("Should not receive incoming sensors or images".into());
        }
    };
    Ok(Some(message))
}

#[repr(u8)]
pub enum MessageKind {
    Sensors = 1,
    BotcamImage = 2,
    ZedImage = 3,
    MlTarget = 4,
    Motors = 5,
    BotcamOn = 6,
    ZedOn = 7,
    LocalizationEstimate = 8,
}

impl TryFrom<u8> for MessageKind {
    type Error = BevyError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::Sensors),
            2 => Ok(Self::BotcamImage),
            3 => Ok(Self::ZedImage),
            4 => Ok(Self::MlTarget),
            5 => Ok(Self::Motors),
            6 => Ok(Self::BotcamOn),
            7 => Ok(Self::ZedOn),
            8 => Ok(Self::LocalizationEstimate),
            _ => Err("Invalid message kind".into()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Dvl {
    pub velocity_a: f32,
    pub velocity_b: f32,
    pub velocity_c: f32,
}

impl Dvl {
    pub fn to_be_bytes(&self) -> [u8; size_of::<Self>()] {
        flatten_array([
            self.velocity_a.to_be_bytes(),
            self.velocity_b.to_be_bytes(),
            self.velocity_c.to_be_bytes(),
        ])
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ImuINS {
    pub theta: [f32; 3],
}

impl ImuINS {
    pub fn to_be_bytes(&self) -> [u8; size_of::<Self>()] {
        flatten_array([
            self.theta[0].to_be_bytes(),
            self.theta[1].to_be_bytes(),
            self.theta[2].to_be_bytes(),
        ])
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ImuPIMU {
    pub dtheta: [f32; 3],
    pub dvel: [f32; 3],
    pub dt: f32,
}

impl ImuPIMU {
    pub fn to_be_bytes(&self) -> [u8; size_of::<Self>()] {
        flatten_array([
            self.dtheta[0].to_be_bytes(),
            self.dtheta[1].to_be_bytes(),
            self.dtheta[2].to_be_bytes(),
            self.dvel[0].to_be_bytes(),
            self.dvel[1].to_be_bytes(),
            self.dvel[2].to_be_bytes(),
            self.dt.to_be_bytes(),
        ])
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SensorMessage {
    pub depth: f32,
    pub dvl: Dvl,
    pub imu_ins: ImuINS,
    pub imu_pimu: ImuPIMU,
}

impl SensorMessage {
    pub fn to_be_bytes(&self) -> [u8; size_of::<Self>()] {
        let mut bytes = [0; size_of::<Self>()];
        macro_rules! copy_field {
            ($field:ident) => {
                bytes[offset_of!(Self, $field)
                    ..offset_of!(Self, $field) + size_of_val(&self.$field)]
                    .copy_from_slice(&self.$field.to_be_bytes());
            };
        }
        copy_field!(depth);
        copy_field!(dvl);
        copy_field!(imu_ins);
        copy_field!(imu_pimu);
        bytes
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum MLTargetKind {
    GateRed = 0,
    GateBlue = 1,
    #[default]
    None = 255,
}

#[repr(C)]
pub struct MLTargetData {
    pub kind: MLTargetKind,
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

pub enum OutgoingMessage {
    Sensors(SensorMessage),
    BotcamImage(SystemTime, Image),
    ZedImage(SystemTime, Image),
    MlTarget(SmallVec<[MLTargetData; 2]>, Vec2),
}

impl OutgoingMessage {
    fn kind(&self) -> MessageKind {
        self.into()
    }

    fn len(&self) -> u64 {
        (1 + match self {
            OutgoingMessage::Sensors(sensors) => std::mem::size_of_val(sensors),
            OutgoingMessage::BotcamImage(_, image) | OutgoingMessage::ZedImage(_, image) => {
                size_of::<f64>() + size_of::<u32>() * 2 + size_of::<u64>() + image.buffer.len()
            }
            OutgoingMessage::MlTarget(targets, _size) => {
                size_of::<u8>()
                    + size_of::<[f32; 2]>()
                    + (size_of::<MLTargetKind>() + size_of::<[f32; 4]>()) * targets.len()
            }
        }) as u64
    }
}

impl From<&OutgoingMessage> for MessageKind {
    fn from(v: &OutgoingMessage) -> Self {
        match v {
            OutgoingMessage::Sensors(..) => Self::Sensors,
            OutgoingMessage::BotcamImage(..) => Self::BotcamImage,
            OutgoingMessage::ZedImage(..) => Self::ZedImage,
            OutgoingMessage::MlTarget(..) => Self::MlTarget,
        }
    }
}

#[derive(Debug, Event)]
pub enum IncomingMessage {
    Motors([f32; 8]),
    BotcamOn(bool),
    ZedOn(bool),
    LocalizationEstimate {
        rotation: Mat3,
        position: Vec3,
        velocity: Vec3,
    },
}

struct CancelCheck;

impl Drop for CancelCheck {
    fn drop(&mut self) {
        MESSAGES_CANCELLED.fetch_add(1, Ordering::Relaxed);
    }
}

pub async fn send(message: OutgoingMessage) -> Result {
    MESSAGES_STARTED.fetch_add(1, Ordering::Relaxed);
    let mut client = Async::<TcpStream>::connect(HAL_INCOMING).await?;
    let cancel = CancelCheck;
    client.write_all(&message.len().to_be_bytes()).await?;
    client.write_all(&[message.kind() as u8]).await?;
    match message {
        OutgoingMessage::Sensors(sensors) => {
            client.write_all(&sensors.to_be_bytes()).await?;
        }
        OutgoingMessage::BotcamImage(time, image) | OutgoingMessage::ZedImage(time, image) => {
            let since_epoch = time
                .duration_since(UNIX_EPOCH)
                .expect("Time should not be before UNIX_EPOCH")
                .as_secs_f64();

            // TODO: image compression
            client.write_all(&since_epoch.to_be_bytes()).await?;
            client.write_all(&image.width.to_be_bytes()).await?;
            client.write_all(&image.height.to_be_bytes()).await?;
            client
                .write_all(&(image.buffer.len() as u64).to_be_bytes())
                .await?;
            client.write_all(&image.buffer).await?;
        }
        OutgoingMessage::MlTarget(targets, size) => {
            client.write_all(&[targets.len() as u8]).await?;
            client.write_all(&size.x.to_be_bytes()).await?;
            client.write_all(&size.y.to_be_bytes()).await?;
            for target in targets {
                client.write_all(&[target.kind as u8]).await?;
                client.write_all(&target.left.to_be_bytes()).await?;
                client.write_all(&target.top.to_be_bytes()).await?;
                client.write_all(&target.right.to_be_bytes()).await?;
                client.write_all(&target.bottom.to_be_bytes()).await?;
            }
        }
    }
    client.flush().await?;
    forget(cancel);
    MESSAGES_FINISHED.fetch_add(1, Ordering::Relaxed);
    Ok(())
}
