#![allow(unused_imports)]

use humantime::format_duration;
use axum::{
    async_trait,
    extract::{Extension, Path},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    AddExtensionLayer, Router,
};
use axum_extra::routing::{Resource, RouterExt};
use axum_liveview::{
    message::Bincode,
    pubsub::{PubSub, PubSubExt},
    LiveView, LiveViewManager, ShouldRender, Subscriptions,
};
use bytesize::ByteSize;
use futures::prelude::*;
use http_stats::HttpStats;
use maud::{html, Markup};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, net::SocketAddr, sync::Arc, time::Duration};
use tokio::time::interval;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::Instrument;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod http_stats;
mod instrument;

#[tokio::main]
async fn main() {
    let (console_layer, console_server) = console_subscriber::Builder::default()
        .publish_interval(Duration::from_secs(1))
        .build();

    tracing_subscriber::registry()
        // TODO(david): per layer filter so we don't get spam from tokio console
        // .with(fmt::layer())
        .with(
            EnvFilter::default()
                .add_directive("tower_http=trace".parse().unwrap())
                .add_directive("demo_app=trace".parse().unwrap())
                .add_directive("axum_liveview=debug".parse().unwrap())
                .add_directive("tokio=trace".parse().unwrap())
                .add_directive("runtime=trace".parse().unwrap()),
        )
        .with(console_layer)
        .init();

    tokio::spawn(async move {
        console_server.serve().await.expect("console server failed");
    });

    let pubsub = axum_liveview::pubsub::InProcess::new();

    let instrument_state = Arc::new(RwLock::new(instrument::State::default()));

    tokio::spawn(instrument::run_client(
        pubsub.clone(),
        instrument_state.clone(),
    ));

    let app = Router::new()
        // .route("/", get(root))
        .route("/", get(root))
        .with(
            Resource::named("tasks")
                .index(tasks_index)
                .show(tasks_show)
                .new(tasks_new),
        )
        .merge(axum_liveview::routes())
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(axum_liveview::layer(pubsub.clone()))
                .layer(AddExtensionLayer::new(pubsub.clone()))
                .layer(AddExtensionLayer::new(instrument_state))
                .layer(http_stats::HttpStatsLayer::new(pubsub))
                .layer(CompressionLayer::new()),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> impl IntoResponse {
    layout(html! {
        p {
            "Welcome!"
        }
    })
}

async fn tasks_index(
    live: LiveViewManager,
    Extension(state): Extension<Arc<RwLock<instrument::State>>>,
) -> impl IntoResponse {
    struct View {
        state: instrument::State,
    }

    impl LiveView for View {
        fn setup(&self, sub: &mut Subscriptions<Self>) {
            sub.on_global("tasks", Self::task_update);
        }

        fn render(&self) -> Markup {
            let table = Table::<(&u64, &instrument::Task)>::new()
                .column("ID", |(id, _)| html! { (id) })
                .column("File", |(_, task)| {
                    html! {
                        @if let Some(location) = &task.location {
                            a href=(format!("/tasks/{}", task.id)) {
                                code {
                                    (location.file)
                                    @if let Some(location) = &task.location {
                                        ":" (location.line) ":" (location.column)
                                    }
                                }
                            }
                        }
                    }
                })
                .column("Fields", |(_, task)| {
                    html! {
                        code {
                            @for (name, value) in &task.fields {
                                (name) "=" (value) "; "
                            }
                        }
                    }
                })
                .column("Total", |(_, task)| {
                    html! {
                        @if let Some(duration) = task.created_at.as_ref().and_then(|t| t.elapsed().ok()) {
                            (format_duration(duration))
                        }
                    }
                })
                .column("Busy", |(_, task)| -> maud::PreEscaped<String> {
                    html! {
                        @if let Some(duration) = task.busy_time {
                            (format_duration(duration))
                        }
                    }
                })
                .column("Last poll ended", |(_, task)| {
                    html! {
                        @if let Some(duration) = task.last_poll_ended.as_ref().and_then(|t| t.elapsed().ok()) {
                            (format_duration(duration)) " ago"
                        }
                    }
                })
                .render(self.state.tasks.iter());

            html! {
                (table)
            }
        }
    }

    impl View {
        async fn task_update(mut self, Bincode(state): Bincode<instrument::State>) -> Self {
            self.state = state;
            self
        }
    }

    let view = View {
        state: state.read().clone(),
    };
    layout(live.embed(view))
}

async fn tasks_show(
    live: LiveViewManager,
    Extension(state): Extension<Arc<RwLock<instrument::State>>>,
    Path(task_id): Path<u64>,
) -> impl IntoResponse {
    struct View {
        task: instrument::Task,
        file: File,
    }

    enum File {
        NotLoaded,
        Loaded(String),
    }

    impl LiveView for View {
        fn setup(&self, sub: &mut Subscriptions<Self>) {
            sub.on_global(&format!("tasks/{}", self.task.id), Self::update_task);
            sub.on("load_file", Self::load_file);
        }

        fn render(&self) -> Markup {
            html! {
                @if self.task.dropped {
                    p {
                        "🚨 Task has been dropped 🚨"
                    }
                }

                div {
                    dl {
                        dt { "Id" }
                        dd { (self.task.id) }

                        dt { "Wakes" }
                        dd { (self.task.wakes) }

                        dt { "Waker clones" }
                        dd { (self.task.waker_clones) }

                        dt { "Waker drops" }
                        dd { (self.task.waker_drops) }

                        dt { "Self wakes" }
                        dd { (self.task.self_wakes) }
                    }
                }

                div {
                    h3 { "Fields" }
                    dl {
                        @for (name, value) in &self.task.fields {
                            dt { (name) }
                            dd { code { (value) } }
                        }
                    }
                }

                div {
                    h3 { "File" }
                    @if let Some(location) = &self.task.location {
                        p {
                            code {
                                (location.file) ":"
                                (location.line) ":"
                                (location.column)
                            }
                        }
                        @match &self.file {
                            File::NotLoaded => {
                                button live-click="load_file" {
                                    "Load code"
                                }
                            }
                            File::Loaded(code) => {
                                code {
                                    pre {
                                        (code)
                                    }
                                }
                            }
                        }
                    } @else {
                        p {
                            "Task has no location"
                        }
                    }
                }
            }
        }
    }

    impl View {
        async fn update_task(mut self, Bincode(task): Bincode<instrument::Task>) -> Self {
            self.task = task;
            self
        }

        async fn load_file(mut self) -> Self {
            let location = if let Some(location) = &self.task.location {
                location
            } else {
                return self;
            };

            let contents = if let Ok(contents) = tokio::fs::read_to_string(&location.file).await {
                contents
            } else {
                return self;
            };

            let start = location.line.checked_sub(6).unwrap_or_default() as usize;
            let lines = contents
                .lines()
                .enumerate()
                .map(|(idx, line)| format!("{} {}", idx + 1, line))
                .skip(start)
                .take(11)
                .collect::<Vec<_>>()
                .join("\n");

            self.file = File::Loaded(lines);

            self
        }
    }

    let (status, html) = if let Some(task) = state.read().tasks.get(&task_id).cloned() {
        let view = View {
            task,
            file: File::NotLoaded,
        };
        (StatusCode::OK, live.embed(view))
    } else {
        (
            StatusCode::NOT_FOUND,
            html! {
                "Task not found"
            },
        )
    };

    (status, layout(html))
}

async fn tasks_new(live: LiveViewManager) -> impl IntoResponse {
    #[derive(Default)]
    struct View {
        name: String,
    }

    impl LiveView for View {
        fn setup(&self, sub: &mut Subscriptions<Self>) {
            sub.on("name-changed", Self::name_changed);
            sub.on("submit", Self::submit);
        }

        fn render(&self) -> Markup {
            html! {
                form {
                    label {
                        "Name"
                        input type="text" value=(self.name) live-input="name-changed" {}
                    }
                    button live-click="submit" {
                        "Start"
                    }
                }
            }
        }
    }

    impl View {
        async fn name_changed(mut self, value: String) -> ShouldRender<Self> {
            self.name = value;
            ShouldRender::No(self)
        }

        async fn submit(mut self) -> Self {
            let name = std::mem::take(&mut self.name);

            tokio::task::Builder::default().name(&name).spawn(async {
                tokio::time::sleep(Duration::from_secs(5)).await;
            });

            self
        }
    }

    layout(live.embed(View::default()))
}

fn layout(html: Markup) -> impl IntoResponse {
    Html(
        html! {
            (maud::DOCTYPE)
            html {
                head {
                    (axum_liveview::assets())
                    style {
                        r#"
                            body {
                                font-family: sans-serif;
                            }
                            a, a:visited {
                                color: black;
                            }
                            table, th, td {
                                border: 1px solid rgb(40, 40, 40);
                                border-collapse: collapse;
                                padding: .5em;
                            }
                            nav {
                                margin-bottom: .5em;
                            }
                        "#
                    }
                }
                body {
                    div {
                        h3 {
                            "Tokio console data, proof of concept"
                        }
                    }
                    nav {
                        a href="/" { "Home" }
                        " - "
                        a href="/tasks" { "Tasks" }
                        " - "
                        a href="/tasks/new" { "New task" }
                    }

                    div {
                        (html)
                    }

                    script {
                        r#"
                        const liveView = new LiveView('localhost', 3000)
                        liveView.connect()
                        "#
                    }
                }
            }
        }
        .into_string(),
    )
}

struct Table<T> {
    cols: Vec<(String, Box<dyn Fn(&T) -> Markup>)>,
    _marker: PhantomData<T>,
}

impl<T> Table<T> {
    fn new() -> Self {
        Self {
            cols: Default::default(),
            _marker: PhantomData,
        }
    }

    fn column<F>(mut self, name: impl Into<String>, f: F) -> Self
    where
        F: Fn(&T) -> Markup + 'static,
    {
        self.cols.push((name.into(), Box::new(f)));
        self
    }

    fn render(&self, items: impl Iterator<Item = T>) -> Markup {
        let headers = self.cols.iter().map(|(col, _)| col).collect::<Vec<_>>();

        let rows = items
            .into_iter()
            .map(|item| self.cols.iter().map(|(_, f)| f(&item)).collect())
            .collect::<Vec<Vec<_>>>();

        html! {
            table {
                thead {
                    tr {
                        @for header in headers {
                            th {
                                (header)
                            }
                        }
                    }
                }
                tbody {
                    @for cells in rows {
                        tr {
                            @for cell in cells {
                                td { (cell) }
                            }
                        }
                    }
                }
            }
        }
    }
}
