use crate::{
    event_data::EventData,
    js_command::JsCommand,
    life_cycle::{UpdateResponse, ViewRequestError, ViewTaskHandle},
    live_view::ViewHandle,
    LiveView,
};
use http::{HeaderMap, Uri};
use serde::Serialize;

pub fn run_live_view<L>(view: L) -> TestViewHandleBuilder<L::Message, L::Error>
where
    L: LiveView,
{
    let view_task_handle = crate::life_cycle::spawn_view(view);

    TestViewHandleBuilder {
        handle: view_task_handle,
        uri: None,
        headers: None,
    }
}

pub struct TestViewHandleBuilder<M, E> {
    handle: ViewTaskHandle<M, E>,
    uri: Option<Uri>,
    headers: Option<HeaderMap>,
}

impl<M, E> TestViewHandleBuilder<M, E> {
    pub fn mount_uri(mut self, uri: Uri) -> Self {
        self.uri = Some(uri);
        self
    }

    pub fn mount_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    pub async fn mount(self) -> Result<TestViewHandle<M, E>, E> {
        // TODO(david): document that the handle passed to mount will have its
        // receiver dropped. All messages need to come from the test. It would
        // probably make testing very hard if random tasks could invoke the view.
        // How should the test access the diffs from those updates?
        let (handle, rx) = ViewHandle::new();
        drop(rx);

        let uri = self.uri.unwrap_or_else(|| "/".parse::<Uri>().unwrap());
        let headers = self.headers.unwrap_or_default();

        match self.handle.mount(uri, headers, handle).await {
            Ok(()) => {}
            Err(ViewRequestError::ViewError(err)) => return Err(err),
            Err(ViewRequestError::ChannelClosed(_)) => unreachable!(),
        }

        Ok(TestViewHandle {
            handle: self.handle,
        })
    }
}

pub struct TestViewHandle<M, E> {
    handle: ViewTaskHandle<M, E>,
}

impl<M, E> TestViewHandle<M, E>
where
    M: Serialize,
{
    pub async fn render(&self) -> String {
        self.handle.render_to_string().await.unwrap()
    }

    pub async fn send(
        &self,
        msg: M,
        data: Option<EventData>,
    ) -> Result<(String, Vec<JsCommand>), E> {
        let js_commands = match self.handle.update(msg, data).await {
            Ok(UpdateResponse::Diff(_) | UpdateResponse::Empty) => Vec::new(),
            Ok(UpdateResponse::JsCommands(cmds) | UpdateResponse::DiffAndJsCommands(_, cmds)) => {
                cmds
            }
            Err(ViewRequestError::ViewError(err)) => return Err(err),
            Err(ViewRequestError::ChannelClosed(_)) => unreachable!(),
        };

        let html = self.handle.render_to_string().await.unwrap();
        Ok((html, js_commands))
    }
}

#[cfg(test)]
mod tests {
    use crate as axum_live_view;
    use crate::event_data::Input;
    use crate::{live_view::Updated, Html};
    use async_trait::async_trait;
    use axum_live_view_macros::html;
    use serde::Deserialize;
    use std::convert::Infallible;

    #[allow(unused_imports)]
    use super::*;

    #[tokio::test]
    async fn test_something() {
        let view = run_live_view(Counter::default()).mount().await.unwrap();

        let html = view.render().await;
        assert!(html.contains('0'));

        let (html, _) = view.send(Msg::Incr, None).await.unwrap();
        assert!(html.contains('1'));

        let (html, _) = view.send(Msg::Incr, None).await.unwrap();
        assert!(html.contains('2'));

        let (html, _) = view.send(Msg::Decr, None).await.unwrap();
        assert!(html.contains('1'));

        let (html, _) = view.send(Msg::Decr, None).await.unwrap();
        assert!(html.contains('0'));

        let (html, _) = view.send(Msg::Decr, None).await.unwrap();
        assert!(html.contains('0'));

        let (html, _) = view
            .send(Msg::IncrBy, Some(Input::String("10".to_owned()).into()))
            .await
            .unwrap();
        assert!(html.contains("10"));
    }

    #[derive(Default, Clone)]
    struct Counter {
        count: u64,
    }

    #[async_trait]
    impl LiveView for Counter {
        type Message = Msg;
        type Error = Infallible;

        async fn update(
            mut self,
            msg: Msg,
            data: Option<EventData>,
        ) -> Result<Updated<Self>, Self::Error> {
            match msg {
                Msg::Incr => self.count += 1,
                Msg::Decr => {
                    if self.count > 0 {
                        self.count -= 1;
                    }
                }
                Msg::IncrBy => {
                    self.count += data
                        .expect("no event data")
                        .as_input()
                        .expect("not input")
                        .as_str()
                        .expect("not string")
                        .parse::<u64>()
                        .expect("parse error");
                }
            }

            Ok(Updated::new(self))
        }

        fn render(&self) -> Html<Self::Message> {
            html! {
                { self.count }
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
    enum Msg {
        Incr,
        Decr,
        IncrBy,
    }
}
