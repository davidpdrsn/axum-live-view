use axum_liveview::{pubsub::PubSub, message::Bincode, PubSubExt};
use console_api::instrument::Update;
use futures::prelude::*;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{Duration, SystemTime},
};

pub async fn run_client<P>(pubsub: P, initial_liveview_state_handle: Arc<RwLock<State>>)
where
    P: PubSub,
{
    let endpoint = format!(
        "http://{}:{}",
        console_subscriber::Server::DEFAULT_IP,
        console_subscriber::Server::DEFAULT_PORT,
    );
    let mut instrument_client =
        console_api::instrument::instrument_client::InstrumentClient::connect(endpoint)
            .await
            .unwrap();

    let mut watch_updates_stream = instrument_client
        .watch_updates(console_api::instrument::InstrumentRequest {})
        .await
        .unwrap()
        .into_inner();

    let mut state = State::default();

    while let Some(update) = watch_updates_stream.next().await {
        let update = update.unwrap();
        handle_update(&pubsub, &mut state, update).await;

        *initial_liveview_state_handle.write() = state.clone();

        let _ = pubsub.broadcast("tasks", Bincode(state.clone())).await;
    }
}

async fn handle_update<P>(pubsub: &P, state: &mut State, update: Update)
where
    P: PubSub,
{
    let Update {
        now,
        task_update,
        resource_update,
        async_op_update,
        new_metadata,
    } = update;

    if let Some(task_update) = task_update {
        for task in task_update.new_tasks {
            let id = task.id.unwrap().id;

            let location = task.location.map(|location| Location {
                file: location.file.unwrap(),
                line: location.line.unwrap(),
                column: location.column.unwrap(),
            });

            let fields = task
                .fields
                .into_iter()
                .filter_map(|field| field.name.zip(field.value))
                .map(|(name, value)| {
                    let name = match name {
                        console_api::field::Name::StrName(name) => name,
                        console_api::field::Name::NameIdx(id) => id.to_string(),
                    };
                    let value = value.to_string();
                    (name, value)
                })
                .collect();

            state.tasks.insert(
                id,
                Task {
                    id,
                    location,
                    fields,
                    wakes: 0,
                    waker_clones: 0,
                    waker_drops: 0,
                    self_wakes: 0,
                    dropped: false,
                    created_at: None,
                    busy_time: None,
                    last_poll_ended: None,
                },
            );
        }

        for (task_id, stats) in &task_update.stats_update {
            if let Some(task) = state.tasks.get_mut(task_id) {
                task.wakes = stats.wakes;
                task.waker_clones = stats.waker_clones;
                task.waker_drops = stats.waker_drops;
                task.self_wakes = stats.self_wakes;
                task.dropped = stats.dropped_at.is_some();

                if let Some(created_at) = stats
                    .created_at
                    .clone()
                    .and_then(|time| SystemTime::try_from(time).ok())
                {
                    task.created_at = Some(created_at);
                }

                if let Some(poll_stats) = &stats.poll_stats {
                    if let Some(busy_time) = poll_stats
                        .busy_time
                        .clone()
                        .and_then(|duration| duration.try_into().ok())
                    {
                        task.busy_time = Some(busy_time);
                    }

                    if let Some(last_poll_ended) = poll_stats
                        .last_poll_ended
                        .clone()
                        .and_then(|time| SystemTime::try_from(time).ok())
                    {
                        task.last_poll_ended = Some(last_poll_ended);
                    }
                }

                let _ = pubsub
                    .broadcast(&format!("tasks/{}", task.id), Bincode(task.clone()))
                    .await;
            }

            if stats.dropped_at.is_some() {
                state.tasks.remove(task_id);
            }
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct State {
    pub tasks: BTreeMap<u64, Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub location: Option<Location>,
    pub fields: BTreeMap<String, String>,
    pub wakes: u64,
    pub waker_clones: u64,
    pub waker_drops: u64,
    pub self_wakes: u64,
    pub dropped: bool,
    pub created_at: Option<SystemTime>,
    pub last_poll_ended: Option<SystemTime>,
    pub busy_time: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file: String,
    pub line: u32,
    pub column: u32,
}
