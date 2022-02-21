use crate::{
    event_data::EventData,
    html::Html,
    live_view::{Updated, ViewHandle},
    LiveView,
};
use async_trait::async_trait;
use axum::http::{HeaderMap, Uri};
use serde::{Deserialize, Serialize};
#[allow(missing_debug_implementations)]
pub struct Combine<V, F> {
    pub(super) views: V,
    pub(super) render: F,
}
#[allow(unreachable_pub, missing_debug_implementations)]
#[derive(PartialEq, Serialize, Deserialize)]
pub enum Either1<T1> {
    T1(T1),
}
#[allow(unreachable_pub, missing_debug_implementations)]
#[derive(PartialEq, Serialize, Deserialize)]
pub enum Either2<T1, T2> {
    T1(T1),
    T2(T2),
}
#[allow(unreachable_pub, missing_debug_implementations)]
#[derive(PartialEq, Serialize, Deserialize)]
pub enum Either3<T1, T2, T3> {
    T1(T1),
    T2(T2),
    T3(T3),
}
#[allow(unreachable_pub, missing_debug_implementations)]
#[derive(PartialEq, Serialize, Deserialize)]
pub enum Either4<T1, T2, T3, T4> {
    T1(T1),
    T2(T2),
    T3(T3),
    T4(T4),
}
#[allow(unreachable_pub, missing_debug_implementations)]
#[derive(PartialEq, Serialize, Deserialize)]
pub enum Either5<T1, T2, T3, T4, T5> {
    T1(T1),
    T2(T2),
    T3(T3),
    T4(T4),
    T5(T5),
}
#[allow(unreachable_pub, missing_debug_implementations)]
#[derive(PartialEq, Serialize, Deserialize)]
pub enum Either6<T1, T2, T3, T4, T5, T6> {
    T1(T1),
    T2(T2),
    T3(T3),
    T4(T4),
    T5(T5),
    T6(T6),
}
#[allow(unreachable_pub, missing_debug_implementations)]
#[derive(PartialEq, Serialize, Deserialize)]
pub enum Either7<T1, T2, T3, T4, T5, T6, T7> {
    T1(T1),
    T2(T2),
    T3(T3),
    T4(T4),
    T5(T5),
    T6(T6),
    T7(T7),
}
#[allow(unreachable_pub, missing_debug_implementations)]
#[derive(PartialEq, Serialize, Deserialize)]
pub enum Either8<T1, T2, T3, T4, T5, T6, T7, T8> {
    T1(T1),
    T2(T2),
    T3(T3),
    T4(T4),
    T5(T5),
    T6(T6),
    T7(T7),
    T8(T8),
}
#[allow(non_snake_case)]
#[async_trait]
impl<F, E, T1> LiveView for Combine<(T1,), F>
where
    T1: LiveView<Error = E>,
    F: Fn(Html<Either1<T1::Message>>) -> Html<Either1<T1::Message>> + Send + Sync + 'static,
    E: std::fmt::Display + Send + Sync + 'static,
{
    type Message = Either1<T1::Message>;
    type Error = E;
    async fn mount(
        &mut self,
        uri: Uri,
        request_headers: &HeaderMap,
        handle: ViewHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        let Self { views: (T1,), .. } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either1::T1),
        )
        .await?;
        Ok(())
    }
    async fn update(
        mut self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        match msg {
            Either1::T1(msg) => {
                let Self {
                    views: (T1,),
                    render,
                } = self;
                let (T1, cmds) = T1.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1,),
                    render,
                })
                .with_all(cmds))
            }
        }
    }
    fn render(&self) -> Html<Self::Message> {
        let Self {
            views: (T1,),
            render,
        } = self;
        render(T1.render().map(Either1::T1))
    }
}
#[allow(non_snake_case)]
#[async_trait]
impl<F, E, T1, T2> LiveView for Combine<(T1, T2), F>
where
    T1: LiveView<Error = E>,
    T2: LiveView<Error = E>,
    F: Fn(
            Html<Either2<T1::Message, T2::Message>>,
            Html<Either2<T1::Message, T2::Message>>,
        ) -> Html<Either2<T1::Message, T2::Message>>
        + Send
        + Sync
        + 'static,
    E: std::fmt::Display + Send + Sync + 'static,
{
    type Message = Either2<T1::Message, T2::Message>;
    type Error = E;
    async fn mount(
        &mut self,
        uri: Uri,
        request_headers: &HeaderMap,
        handle: ViewHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        let Self {
            views: (T1, T2), ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either2::T1),
        )
        .await?;
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either2::T2),
        )
        .await?;
        Ok(())
    }
    async fn update(
        mut self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        match msg {
            Either2::T1(msg) => {
                let Self {
                    views: (T1, T2),
                    render,
                } = self;
                let (T1, cmds) = T1.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2),
                    render,
                })
                .with_all(cmds))
            }
            Either2::T2(msg) => {
                let Self {
                    views: (T1, T2),
                    render,
                } = self;
                let (T2, cmds) = T2.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2),
                    render,
                })
                .with_all(cmds))
            }
        }
    }
    fn render(&self) -> Html<Self::Message> {
        let Self {
            views: (T1, T2),
            render,
        } = self;
        render(T1.render().map(Either2::T1), T2.render().map(Either2::T2))
    }
}
#[allow(non_snake_case)]
#[async_trait]
impl<F, E, T1, T2, T3> LiveView for Combine<(T1, T2, T3), F>
where
    T1: LiveView<Error = E>,
    T2: LiveView<Error = E>,
    T3: LiveView<Error = E>,
    F: Fn(
            Html<Either3<T1::Message, T2::Message, T3::Message>>,
            Html<Either3<T1::Message, T2::Message, T3::Message>>,
            Html<Either3<T1::Message, T2::Message, T3::Message>>,
        ) -> Html<Either3<T1::Message, T2::Message, T3::Message>>
        + Send
        + Sync
        + 'static,
    E: std::fmt::Display + Send + Sync + 'static,
{
    type Message = Either3<T1::Message, T2::Message, T3::Message>;
    type Error = E;
    async fn mount(
        &mut self,
        uri: Uri,
        request_headers: &HeaderMap,
        handle: ViewHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        let Self {
            views: (T1, T2, T3),
            ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either3::T1),
        )
        .await?;
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either3::T2),
        )
        .await?;
        T3.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either3::T3),
        )
        .await?;
        Ok(())
    }
    async fn update(
        mut self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        match msg {
            Either3::T1(msg) => {
                let Self {
                    views: (T1, T2, T3),
                    render,
                } = self;
                let (T1, cmds) = T1.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3),
                    render,
                })
                .with_all(cmds))
            }
            Either3::T2(msg) => {
                let Self {
                    views: (T1, T2, T3),
                    render,
                } = self;
                let (T2, cmds) = T2.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3),
                    render,
                })
                .with_all(cmds))
            }
            Either3::T3(msg) => {
                let Self {
                    views: (T1, T2, T3),
                    render,
                } = self;
                let (T3, cmds) = T3.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3),
                    render,
                })
                .with_all(cmds))
            }
        }
    }
    fn render(&self) -> Html<Self::Message> {
        let Self {
            views: (T1, T2, T3),
            render,
        } = self;
        render(
            T1.render().map(Either3::T1),
            T2.render().map(Either3::T2),
            T3.render().map(Either3::T3),
        )
    }
}
#[allow(non_snake_case)]
#[async_trait]
impl<F, E, T1, T2, T3, T4> LiveView for Combine<(T1, T2, T3, T4), F>
where
    T1: LiveView<Error = E>,
    T2: LiveView<Error = E>,
    T3: LiveView<Error = E>,
    T4: LiveView<Error = E>,
    F: Fn(
            Html<Either4<T1::Message, T2::Message, T3::Message, T4::Message>>,
            Html<Either4<T1::Message, T2::Message, T3::Message, T4::Message>>,
            Html<Either4<T1::Message, T2::Message, T3::Message, T4::Message>>,
            Html<Either4<T1::Message, T2::Message, T3::Message, T4::Message>>,
        ) -> Html<Either4<T1::Message, T2::Message, T3::Message, T4::Message>>
        + Send
        + Sync
        + 'static,
    E: std::fmt::Display + Send + Sync + 'static,
{
    type Message = Either4<T1::Message, T2::Message, T3::Message, T4::Message>;
    type Error = E;
    async fn mount(
        &mut self,
        uri: Uri,
        request_headers: &HeaderMap,
        handle: ViewHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        let Self {
            views: (T1, T2, T3, T4),
            ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either4::T1),
        )
        .await?;
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either4::T2),
        )
        .await?;
        T3.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either4::T3),
        )
        .await?;
        T4.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either4::T4),
        )
        .await?;
        Ok(())
    }
    async fn update(
        mut self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        match msg {
            Either4::T1(msg) => {
                let Self {
                    views: (T1, T2, T3, T4),
                    render,
                } = self;
                let (T1, cmds) = T1.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4),
                    render,
                })
                .with_all(cmds))
            }
            Either4::T2(msg) => {
                let Self {
                    views: (T1, T2, T3, T4),
                    render,
                } = self;
                let (T2, cmds) = T2.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4),
                    render,
                })
                .with_all(cmds))
            }
            Either4::T3(msg) => {
                let Self {
                    views: (T1, T2, T3, T4),
                    render,
                } = self;
                let (T3, cmds) = T3.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4),
                    render,
                })
                .with_all(cmds))
            }
            Either4::T4(msg) => {
                let Self {
                    views: (T1, T2, T3, T4),
                    render,
                } = self;
                let (T4, cmds) = T4.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4),
                    render,
                })
                .with_all(cmds))
            }
        }
    }
    fn render(&self) -> Html<Self::Message> {
        let Self {
            views: (T1, T2, T3, T4),
            render,
        } = self;
        render(
            T1.render().map(Either4::T1),
            T2.render().map(Either4::T2),
            T3.render().map(Either4::T3),
            T4.render().map(Either4::T4),
        )
    }
}
#[allow(non_snake_case)]
#[async_trait]
impl<F, E, T1, T2, T3, T4, T5> LiveView for Combine<(T1, T2, T3, T4, T5), F>
where
    T1: LiveView<Error = E>,
    T2: LiveView<Error = E>,
    T3: LiveView<Error = E>,
    T4: LiveView<Error = E>,
    T5: LiveView<Error = E>,
    F: Fn(
            Html<Either5<T1::Message, T2::Message, T3::Message, T4::Message, T5::Message>>,
            Html<Either5<T1::Message, T2::Message, T3::Message, T4::Message, T5::Message>>,
            Html<Either5<T1::Message, T2::Message, T3::Message, T4::Message, T5::Message>>,
            Html<Either5<T1::Message, T2::Message, T3::Message, T4::Message, T5::Message>>,
            Html<Either5<T1::Message, T2::Message, T3::Message, T4::Message, T5::Message>>,
        )
            -> Html<Either5<T1::Message, T2::Message, T3::Message, T4::Message, T5::Message>>
        + Send
        + Sync
        + 'static,
    E: std::fmt::Display + Send + Sync + 'static,
{
    type Message = Either5<T1::Message, T2::Message, T3::Message, T4::Message, T5::Message>;
    type Error = E;
    async fn mount(
        &mut self,
        uri: Uri,
        request_headers: &HeaderMap,
        handle: ViewHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        let Self {
            views: (T1, T2, T3, T4, T5),
            ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either5::T1),
        )
        .await?;
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either5::T2),
        )
        .await?;
        T3.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either5::T3),
        )
        .await?;
        T4.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either5::T4),
        )
        .await?;
        T5.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either5::T5),
        )
        .await?;
        Ok(())
    }
    async fn update(
        mut self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        match msg {
            Either5::T1(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                } = self;
                let (T1, cmds) = T1.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                })
                .with_all(cmds))
            }
            Either5::T2(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                } = self;
                let (T2, cmds) = T2.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                })
                .with_all(cmds))
            }
            Either5::T3(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                } = self;
                let (T3, cmds) = T3.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                })
                .with_all(cmds))
            }
            Either5::T4(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                } = self;
                let (T4, cmds) = T4.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                })
                .with_all(cmds))
            }
            Either5::T5(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                } = self;
                let (T5, cmds) = T5.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                })
                .with_all(cmds))
            }
        }
    }
    fn render(&self) -> Html<Self::Message> {
        let Self {
            views: (T1, T2, T3, T4, T5),
            render,
        } = self;
        render(
            T1.render().map(Either5::T1),
            T2.render().map(Either5::T2),
            T3.render().map(Either5::T3),
            T4.render().map(Either5::T4),
            T5.render().map(Either5::T5),
        )
    }
}
#[allow(non_snake_case)]
#[async_trait]
impl<F, E, T1, T2, T3, T4, T5, T6> LiveView for Combine<(T1, T2, T3, T4, T5, T6), F>
where
    T1: LiveView<Error = E>,
    T2: LiveView<Error = E>,
    T3: LiveView<Error = E>,
    T4: LiveView<Error = E>,
    T5: LiveView<Error = E>,
    T6: LiveView<Error = E>,
    F: Fn(
            Html<
                Either6<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                >,
            >,
            Html<
                Either6<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                >,
            >,
            Html<
                Either6<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                >,
            >,
            Html<
                Either6<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                >,
            >,
            Html<
                Either6<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                >,
            >,
            Html<
                Either6<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                >,
            >,
        ) -> Html<
            Either6<T1::Message, T2::Message, T3::Message, T4::Message, T5::Message, T6::Message>,
        > + Send
        + Sync
        + 'static,
    E: std::fmt::Display + Send + Sync + 'static,
{
    type Message =
        Either6<T1::Message, T2::Message, T3::Message, T4::Message, T5::Message, T6::Message>;
    type Error = E;
    async fn mount(
        &mut self,
        uri: Uri,
        request_headers: &HeaderMap,
        handle: ViewHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        let Self {
            views: (T1, T2, T3, T4, T5, T6),
            ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either6::T1),
        )
        .await?;
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either6::T2),
        )
        .await?;
        T3.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either6::T3),
        )
        .await?;
        T4.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either6::T4),
        )
        .await?;
        T5.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either6::T5),
        )
        .await?;
        T6.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either6::T6),
        )
        .await?;
        Ok(())
    }
    async fn update(
        mut self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        match msg {
            Either6::T1(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                } = self;
                let (T1, cmds) = T1.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                })
                .with_all(cmds))
            }
            Either6::T2(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                } = self;
                let (T2, cmds) = T2.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                })
                .with_all(cmds))
            }
            Either6::T3(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                } = self;
                let (T3, cmds) = T3.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                })
                .with_all(cmds))
            }
            Either6::T4(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                } = self;
                let (T4, cmds) = T4.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                })
                .with_all(cmds))
            }
            Either6::T5(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                } = self;
                let (T5, cmds) = T5.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                })
                .with_all(cmds))
            }
            Either6::T6(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                } = self;
                let (T6, cmds) = T6.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                })
                .with_all(cmds))
            }
        }
    }
    fn render(&self) -> Html<Self::Message> {
        let Self {
            views: (T1, T2, T3, T4, T5, T6),
            render,
        } = self;
        render(
            T1.render().map(Either6::T1),
            T2.render().map(Either6::T2),
            T3.render().map(Either6::T3),
            T4.render().map(Either6::T4),
            T5.render().map(Either6::T5),
            T6.render().map(Either6::T6),
        )
    }
}
#[allow(non_snake_case)]
#[async_trait]
impl<F, E, T1, T2, T3, T4, T5, T6, T7> LiveView for Combine<(T1, T2, T3, T4, T5, T6, T7), F>
where
    T1: LiveView<Error = E>,
    T2: LiveView<Error = E>,
    T3: LiveView<Error = E>,
    T4: LiveView<Error = E>,
    T5: LiveView<Error = E>,
    T6: LiveView<Error = E>,
    T7: LiveView<Error = E>,
    F: Fn(
            Html<
                Either7<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                >,
            >,
            Html<
                Either7<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                >,
            >,
            Html<
                Either7<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                >,
            >,
            Html<
                Either7<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                >,
            >,
            Html<
                Either7<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                >,
            >,
            Html<
                Either7<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                >,
            >,
            Html<
                Either7<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                >,
            >,
        ) -> Html<
            Either7<
                T1::Message,
                T2::Message,
                T3::Message,
                T4::Message,
                T5::Message,
                T6::Message,
                T7::Message,
            >,
        > + Send
        + Sync
        + 'static,
    E: std::fmt::Display + Send + Sync + 'static,
{
    type Message = Either7<
        T1::Message,
        T2::Message,
        T3::Message,
        T4::Message,
        T5::Message,
        T6::Message,
        T7::Message,
    >;
    type Error = E;
    async fn mount(
        &mut self,
        uri: Uri,
        request_headers: &HeaderMap,
        handle: ViewHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        let Self {
            views: (T1, T2, T3, T4, T5, T6, T7),
            ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T1),
        )
        .await?;
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T2),
        )
        .await?;
        T3.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T3),
        )
        .await?;
        T4.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T4),
        )
        .await?;
        T5.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T5),
        )
        .await?;
        T6.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T6),
        )
        .await?;
        T7.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T7),
        )
        .await?;
        Ok(())
    }
    async fn update(
        mut self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        match msg {
            Either7::T1(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let (T1, cmds) = T1.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                })
                .with_all(cmds))
            }
            Either7::T2(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let (T2, cmds) = T2.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                })
                .with_all(cmds))
            }
            Either7::T3(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let (T3, cmds) = T3.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                })
                .with_all(cmds))
            }
            Either7::T4(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let (T4, cmds) = T4.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                })
                .with_all(cmds))
            }
            Either7::T5(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let (T5, cmds) = T5.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                })
                .with_all(cmds))
            }
            Either7::T6(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let (T6, cmds) = T6.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                })
                .with_all(cmds))
            }
            Either7::T7(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let (T7, cmds) = T7.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                })
                .with_all(cmds))
            }
        }
    }
    fn render(&self) -> Html<Self::Message> {
        let Self {
            views: (T1, T2, T3, T4, T5, T6, T7),
            render,
        } = self;
        render(
            T1.render().map(Either7::T1),
            T2.render().map(Either7::T2),
            T3.render().map(Either7::T3),
            T4.render().map(Either7::T4),
            T5.render().map(Either7::T5),
            T6.render().map(Either7::T6),
            T7.render().map(Either7::T7),
        )
    }
}
#[allow(non_snake_case)]
#[async_trait]
impl<F, E, T1, T2, T3, T4, T5, T6, T7, T8> LiveView for Combine<(T1, T2, T3, T4, T5, T6, T7, T8), F>
where
    T1: LiveView<Error = E>,
    T2: LiveView<Error = E>,
    T3: LiveView<Error = E>,
    T4: LiveView<Error = E>,
    T5: LiveView<Error = E>,
    T6: LiveView<Error = E>,
    T7: LiveView<Error = E>,
    T8: LiveView<Error = E>,
    F: Fn(
            Html<
                Either8<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                    T8::Message,
                >,
            >,
            Html<
                Either8<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                    T8::Message,
                >,
            >,
            Html<
                Either8<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                    T8::Message,
                >,
            >,
            Html<
                Either8<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                    T8::Message,
                >,
            >,
            Html<
                Either8<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                    T8::Message,
                >,
            >,
            Html<
                Either8<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                    T8::Message,
                >,
            >,
            Html<
                Either8<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                    T8::Message,
                >,
            >,
            Html<
                Either8<
                    T1::Message,
                    T2::Message,
                    T3::Message,
                    T4::Message,
                    T5::Message,
                    T6::Message,
                    T7::Message,
                    T8::Message,
                >,
            >,
        ) -> Html<
            Either8<
                T1::Message,
                T2::Message,
                T3::Message,
                T4::Message,
                T5::Message,
                T6::Message,
                T7::Message,
                T8::Message,
            >,
        > + Send
        + Sync
        + 'static,
    E: std::fmt::Display + Send + Sync + 'static,
{
    type Message = Either8<
        T1::Message,
        T2::Message,
        T3::Message,
        T4::Message,
        T5::Message,
        T6::Message,
        T7::Message,
        T8::Message,
    >;
    type Error = E;
    async fn mount(
        &mut self,
        uri: Uri,
        request_headers: &HeaderMap,
        handle: ViewHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        let Self {
            views: (T1, T2, T3, T4, T5, T6, T7, T8),
            ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T1),
        )
        .await?;
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T2),
        )
        .await?;
        T3.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T3),
        )
        .await?;
        T4.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T4),
        )
        .await?;
        T5.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T5),
        )
        .await?;
        T6.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T6),
        )
        .await?;
        T7.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T7),
        )
        .await?;
        T8.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T8),
        )
        .await?;
        Ok(())
    }
    async fn update(
        mut self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        match msg {
            Either8::T1(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let (T1, cmds) = T1.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                })
                .with_all(cmds))
            }
            Either8::T2(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let (T2, cmds) = T2.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                })
                .with_all(cmds))
            }
            Either8::T3(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let (T3, cmds) = T3.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                })
                .with_all(cmds))
            }
            Either8::T4(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let (T4, cmds) = T4.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                })
                .with_all(cmds))
            }
            Either8::T5(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let (T5, cmds) = T5.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                })
                .with_all(cmds))
            }
            Either8::T6(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let (T6, cmds) = T6.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                })
                .with_all(cmds))
            }
            Either8::T7(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let (T7, cmds) = T7.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                })
                .with_all(cmds))
            }
            Either8::T8(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let (T8, cmds) = T8.update(msg, data).await?.into_parts();
                Ok(Updated::new(Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                })
                .with_all(cmds))
            }
        }
    }
    fn render(&self) -> Html<Self::Message> {
        let Self {
            views: (T1, T2, T3, T4, T5, T6, T7, T8),
            render,
        } = self;
        render(
            T1.render().map(Either8::T1),
            T2.render().map(Either8::T2),
            T3.render().map(Either8::T3),
            T4.render().map(Either8::T4),
            T5.render().map(Either8::T5),
            T6.render().map(Either8::T6),
            T7.render().map(Either8::T7),
            T8.render().map(Either8::T8),
        )
    }
}
