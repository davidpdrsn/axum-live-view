use crate::life_cycle::MessageFromSocketData;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum EventData {
    FormSubmit(FormSubmit),
    FormChange(FormChange),
    InputChange(InputChange),
    Key(Key),
    Mouse(Mouse),
}

impl From<MessageFromSocketData> for Option<EventData> {
    fn from(data: MessageFromSocketData) -> Self {
        match data {
            MessageFromSocketData::Click
            | MessageFromSocketData::WindowFocus
            | MessageFromSocketData::WindowBlur
            | MessageFromSocketData::None => None,
            MessageFromSocketData::FormSubmit { query } => {
                Some(EventData::FormSubmit(FormSubmit { query }))
            }
            MessageFromSocketData::FormChange { query } => {
                Some(EventData::FormChange(FormChange { query }))
            }
            MessageFromSocketData::InputChange { value } => {
                Some(EventData::InputChange(InputChange { value }))
            }
            MessageFromSocketData::Key {
                key,
                code,
                alt,
                ctrl,
                shift,
                meta,
            } => Some(EventData::Key(Key {
                key,
                code,
                alt,
                ctrl,
                shift,
                meta,
            })),
            MessageFromSocketData::Mouse {
                client_x,
                client_y,
                page_x,
                page_y,
                offset_x,
                offset_y,
                movement_x,
                movement_y,
                screen_x,
                screen_y,
            } => Some(EventData::Mouse(Mouse {
                client_x,
                client_y,
                page_x,
                page_y,
                offset_x,
                offset_y,
                movement_x,
                movement_y,
                screen_x,
                screen_y,
            })),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormSubmit {
    query: String,
}

impl FormSubmit {
    pub fn query(&self) -> &str {
        &self.query
    }
}

#[derive(Debug, Clone)]
pub struct FormChange {
    query: String,
}

impl FormChange {
    pub fn query(&self) -> &str {
        &self.query
    }
}

#[derive(Debug, Clone)]
pub struct InputChange {
    value: String,
}

impl InputChange {
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone)]
pub struct Key {
    key: String,
    code: String,
    alt: bool,
    ctrl: bool,
    shift: bool,
    meta: bool,
}

impl Key {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn alt(&self) -> bool {
        self.alt
    }

    pub fn ctrl(&self) -> bool {
        self.ctrl
    }

    pub fn shift(&self) -> bool {
        self.shift
    }

    pub fn meta(&self) -> bool {
        self.meta
    }
}

#[derive(Debug, Clone)]
pub struct Mouse {
    client_x: f64,
    client_y: f64,
    page_x: f64,
    page_y: f64,
    offset_x: f64,
    offset_y: f64,
    movement_x: f64,
    movement_y: f64,
    screen_x: f64,
    screen_y: f64,
}

impl Mouse {
    pub fn client_x(&self) -> f64 {
        self.client_x
    }

    pub fn client_y(&self) -> f64 {
        self.client_y
    }

    pub fn page_x(&self) -> f64 {
        self.page_x
    }

    pub fn page_y(&self) -> f64 {
        self.page_y
    }

    pub fn offset_x(&self) -> f64 {
        self.offset_x
    }

    pub fn offset_y(&self) -> f64 {
        self.offset_y
    }

    pub fn movement_x(&self) -> f64 {
        self.movement_x
    }

    pub fn movement_y(&self) -> f64 {
        self.movement_y
    }

    pub fn screen_x(&self) -> f64 {
        self.screen_x
    }

    pub fn screen_y(&self) -> f64 {
        self.screen_y
    }
}
