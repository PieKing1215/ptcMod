#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum EventType {
    Null,
    On,
    Key,
    PanVolume,
    Velocity,
    Volume,
    Portament,
    BeatClock,
    BeatTempo,
    BeatNum,
    Repeat,
    Last,
    VoiceNo,
    GroupNo,
    Tuning,
    PanTime,
}

#[repr(C)]
#[derive(Debug)]
pub struct Event {
    pub kind: EventType,
    pub unit: u8,
    pub reserve1: u8,
    pub reserve2: u8,
    pub value: i32,
    pub clock: i32,
    pub prev: *mut Event,
    pub next: *mut Event,
}

#[repr(C)]
pub struct EventList {
    pub alloc_num: i32,
    pub events: *mut Event,
    pub start: *mut Event,
    pub linear_index: i32,
}
