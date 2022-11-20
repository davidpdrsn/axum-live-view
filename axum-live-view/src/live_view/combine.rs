use crate::{
    event_data::EventData,
    html::Html,
    live_view::{Updated, ViewHandle},
    LiveView,
};
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
impl<F, T1> LiveView for Combine<(T1,), F>
where
    T1: LiveView,
    F: Fn(Html<Either1<T1::Message>>) -> Html<Either1<T1::Message>> + Send + Sync + 'static,
{
    type Message = Either1<T1::Message>;
    fn mount(&mut self, uri: Uri, request_headers: &HeaderMap, handle: ViewHandle<Self::Message>) {
        let Self { views: (T1,), .. } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either1::T1),
        );
    }
    fn update(mut self, msg: Self::Message, data: Option<EventData>) -> Updated<Self> {
        match msg {
            Either1::T1(msg) => {
                let Self {
                    views: (T1,),
                    render,
                } = self;
                let Updated {
                    live_view: T1,
                    js_commands,
                    spawns,
                } = T1.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either1::T1(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1,),
                        render,
                    },
                    js_commands,
                    spawns,
                }
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
impl<F, T1, T2> LiveView for Combine<(T1, T2), F>
where
    T1: LiveView,
    T2: LiveView,
    F: Fn(
            Html<Either2<T1::Message, T2::Message>>,
            Html<Either2<T1::Message, T2::Message>>,
        ) -> Html<Either2<T1::Message, T2::Message>>
        + Send
        + Sync
        + 'static,
{
    type Message = Either2<T1::Message, T2::Message>;
    fn mount(&mut self, uri: Uri, request_headers: &HeaderMap, handle: ViewHandle<Self::Message>) {
        let Self {
            views: (T1, T2), ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either2::T1),
        );
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either2::T2),
        );
    }
    fn update(mut self, msg: Self::Message, data: Option<EventData>) -> Updated<Self> {
        match msg {
            Either2::T1(msg) => {
                let Self {
                    views: (T1, T2),
                    render,
                } = self;
                let Updated {
                    live_view: T1,
                    js_commands,
                    spawns,
                } = T1.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either2::T1(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either2::T2(msg) => {
                let Self {
                    views: (T1, T2),
                    render,
                } = self;
                let Updated {
                    live_view: T2,
                    js_commands,
                    spawns,
                } = T2.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either2::T2(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2),
                        render,
                    },
                    js_commands,
                    spawns,
                }
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
impl<F, T1, T2, T3> LiveView for Combine<(T1, T2, T3), F>
where
    T1: LiveView,
    T2: LiveView,
    T3: LiveView,
    F: Fn(
            Html<Either3<T1::Message, T2::Message, T3::Message>>,
            Html<Either3<T1::Message, T2::Message, T3::Message>>,
            Html<Either3<T1::Message, T2::Message, T3::Message>>,
        ) -> Html<Either3<T1::Message, T2::Message, T3::Message>>
        + Send
        + Sync
        + 'static,
{
    type Message = Either3<T1::Message, T2::Message, T3::Message>;
    fn mount(&mut self, uri: Uri, request_headers: &HeaderMap, handle: ViewHandle<Self::Message>) {
        let Self {
            views: (T1, T2, T3),
            ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either3::T1),
        );
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either3::T2),
        );
        T3.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either3::T3),
        );
    }
    fn update(mut self, msg: Self::Message, data: Option<EventData>) -> Updated<Self> {
        match msg {
            Either3::T1(msg) => {
                let Self {
                    views: (T1, T2, T3),
                    render,
                } = self;
                let Updated {
                    live_view: T1,
                    js_commands,
                    spawns,
                } = T1.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either3::T1(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either3::T2(msg) => {
                let Self {
                    views: (T1, T2, T3),
                    render,
                } = self;
                let Updated {
                    live_view: T2,
                    js_commands,
                    spawns,
                } = T2.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either3::T2(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either3::T3(msg) => {
                let Self {
                    views: (T1, T2, T3),
                    render,
                } = self;
                let Updated {
                    live_view: T3,
                    js_commands,
                    spawns,
                } = T3.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either3::T3(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3),
                        render,
                    },
                    js_commands,
                    spawns,
                }
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
impl<F, T1, T2, T3, T4> LiveView for Combine<(T1, T2, T3, T4), F>
where
    T1: LiveView,
    T2: LiveView,
    T3: LiveView,
    T4: LiveView,
    F: Fn(
            Html<Either4<T1::Message, T2::Message, T3::Message, T4::Message>>,
            Html<Either4<T1::Message, T2::Message, T3::Message, T4::Message>>,
            Html<Either4<T1::Message, T2::Message, T3::Message, T4::Message>>,
            Html<Either4<T1::Message, T2::Message, T3::Message, T4::Message>>,
        ) -> Html<Either4<T1::Message, T2::Message, T3::Message, T4::Message>>
        + Send
        + Sync
        + 'static,
{
    type Message = Either4<T1::Message, T2::Message, T3::Message, T4::Message>;
    fn mount(&mut self, uri: Uri, request_headers: &HeaderMap, handle: ViewHandle<Self::Message>) {
        let Self {
            views: (T1, T2, T3, T4),
            ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either4::T1),
        );
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either4::T2),
        );
        T3.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either4::T3),
        );
        T4.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either4::T4),
        );
    }
    fn update(mut self, msg: Self::Message, data: Option<EventData>) -> Updated<Self> {
        match msg {
            Either4::T1(msg) => {
                let Self {
                    views: (T1, T2, T3, T4),
                    render,
                } = self;
                let Updated {
                    live_view: T1,
                    js_commands,
                    spawns,
                } = T1.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either4::T1(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either4::T2(msg) => {
                let Self {
                    views: (T1, T2, T3, T4),
                    render,
                } = self;
                let Updated {
                    live_view: T2,
                    js_commands,
                    spawns,
                } = T2.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either4::T2(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either4::T3(msg) => {
                let Self {
                    views: (T1, T2, T3, T4),
                    render,
                } = self;
                let Updated {
                    live_view: T3,
                    js_commands,
                    spawns,
                } = T3.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either4::T3(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either4::T4(msg) => {
                let Self {
                    views: (T1, T2, T3, T4),
                    render,
                } = self;
                let Updated {
                    live_view: T4,
                    js_commands,
                    spawns,
                } = T4.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either4::T4(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4),
                        render,
                    },
                    js_commands,
                    spawns,
                }
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
impl<F, T1, T2, T3, T4, T5> LiveView for Combine<(T1, T2, T3, T4, T5), F>
where
    T1: LiveView,
    T2: LiveView,
    T3: LiveView,
    T4: LiveView,
    T5: LiveView,
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
{
    type Message = Either5<T1::Message, T2::Message, T3::Message, T4::Message, T5::Message>;
    fn mount(&mut self, uri: Uri, request_headers: &HeaderMap, handle: ViewHandle<Self::Message>) {
        let Self {
            views: (T1, T2, T3, T4, T5),
            ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either5::T1),
        );
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either5::T2),
        );
        T3.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either5::T3),
        );
        T4.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either5::T4),
        );
        T5.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either5::T5),
        );
    }
    fn update(mut self, msg: Self::Message, data: Option<EventData>) -> Updated<Self> {
        match msg {
            Either5::T1(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                } = self;
                let Updated {
                    live_view: T1,
                    js_commands,
                    spawns,
                } = T1.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either5::T1(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either5::T2(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                } = self;
                let Updated {
                    live_view: T2,
                    js_commands,
                    spawns,
                } = T2.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either5::T2(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either5::T3(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                } = self;
                let Updated {
                    live_view: T3,
                    js_commands,
                    spawns,
                } = T3.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either5::T3(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either5::T4(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                } = self;
                let Updated {
                    live_view: T4,
                    js_commands,
                    spawns,
                } = T4.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either5::T4(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either5::T5(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5),
                    render,
                } = self;
                let Updated {
                    live_view: T5,
                    js_commands,
                    spawns,
                } = T5.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either5::T5(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5),
                        render,
                    },
                    js_commands,
                    spawns,
                }
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
impl<F, T1, T2, T3, T4, T5, T6> LiveView for Combine<(T1, T2, T3, T4, T5, T6), F>
where
    T1: LiveView,
    T2: LiveView,
    T3: LiveView,
    T4: LiveView,
    T5: LiveView,
    T6: LiveView,
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
{
    type Message =
        Either6<T1::Message, T2::Message, T3::Message, T4::Message, T5::Message, T6::Message>;
    fn mount(&mut self, uri: Uri, request_headers: &HeaderMap, handle: ViewHandle<Self::Message>) {
        let Self {
            views: (T1, T2, T3, T4, T5, T6),
            ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either6::T1),
        );
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either6::T2),
        );
        T3.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either6::T3),
        );
        T4.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either6::T4),
        );
        T5.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either6::T5),
        );
        T6.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either6::T6),
        );
    }
    fn update(mut self, msg: Self::Message, data: Option<EventData>) -> Updated<Self> {
        match msg {
            Either6::T1(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                } = self;
                let Updated {
                    live_view: T1,
                    js_commands,
                    spawns,
                } = T1.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either6::T1(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either6::T2(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                } = self;
                let Updated {
                    live_view: T2,
                    js_commands,
                    spawns,
                } = T2.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either6::T2(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either6::T3(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                } = self;
                let Updated {
                    live_view: T3,
                    js_commands,
                    spawns,
                } = T3.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either6::T3(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either6::T4(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                } = self;
                let Updated {
                    live_view: T4,
                    js_commands,
                    spawns,
                } = T4.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either6::T4(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either6::T5(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                } = self;
                let Updated {
                    live_view: T5,
                    js_commands,
                    spawns,
                } = T5.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either6::T5(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either6::T6(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6),
                    render,
                } = self;
                let Updated {
                    live_view: T6,
                    js_commands,
                    spawns,
                } = T6.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either6::T6(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6),
                        render,
                    },
                    js_commands,
                    spawns,
                }
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
impl<F, T1, T2, T3, T4, T5, T6, T7> LiveView for Combine<(T1, T2, T3, T4, T5, T6, T7), F>
where
    T1: LiveView,
    T2: LiveView,
    T3: LiveView,
    T4: LiveView,
    T5: LiveView,
    T6: LiveView,
    T7: LiveView,
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
    fn mount(&mut self, uri: Uri, request_headers: &HeaderMap, handle: ViewHandle<Self::Message>) {
        let Self {
            views: (T1, T2, T3, T4, T5, T6, T7),
            ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T1),
        );
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T2),
        );
        T3.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T3),
        );
        T4.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T4),
        );
        T5.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T5),
        );
        T6.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T6),
        );
        T7.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either7::T7),
        );
    }
    fn update(mut self, msg: Self::Message, data: Option<EventData>) -> Updated<Self> {
        match msg {
            Either7::T1(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let Updated {
                    live_view: T1,
                    js_commands,
                    spawns,
                } = T1.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either7::T1(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either7::T2(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let Updated {
                    live_view: T2,
                    js_commands,
                    spawns,
                } = T2.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either7::T2(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either7::T3(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let Updated {
                    live_view: T3,
                    js_commands,
                    spawns,
                } = T3.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either7::T3(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either7::T4(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let Updated {
                    live_view: T4,
                    js_commands,
                    spawns,
                } = T4.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either7::T4(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either7::T5(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let Updated {
                    live_view: T5,
                    js_commands,
                    spawns,
                } = T5.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either7::T5(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either7::T6(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let Updated {
                    live_view: T6,
                    js_commands,
                    spawns,
                } = T6.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either7::T6(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either7::T7(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7),
                    render,
                } = self;
                let Updated {
                    live_view: T7,
                    js_commands,
                    spawns,
                } = T7.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either7::T7(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7),
                        render,
                    },
                    js_commands,
                    spawns,
                }
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
impl<F, T1, T2, T3, T4, T5, T6, T7, T8> LiveView for Combine<(T1, T2, T3, T4, T5, T6, T7, T8), F>
where
    T1: LiveView,
    T2: LiveView,
    T3: LiveView,
    T4: LiveView,
    T5: LiveView,
    T6: LiveView,
    T7: LiveView,
    T8: LiveView,
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
    fn mount(&mut self, uri: Uri, request_headers: &HeaderMap, handle: ViewHandle<Self::Message>) {
        let Self {
            views: (T1, T2, T3, T4, T5, T6, T7, T8),
            ..
        } = self;
        T1.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T1),
        );
        T2.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T2),
        );
        T3.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T3),
        );
        T4.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T4),
        );
        T5.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T5),
        );
        T6.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T6),
        );
        T7.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T7),
        );
        T8.mount(
            uri.clone(),
            request_headers,
            handle.clone().with(Either8::T8),
        );
    }
    fn update(mut self, msg: Self::Message, data: Option<EventData>) -> Updated<Self> {
        match msg {
            Either8::T1(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let Updated {
                    live_view: T1,
                    js_commands,
                    spawns,
                } = T1.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either8::T1(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7, T8),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either8::T2(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let Updated {
                    live_view: T2,
                    js_commands,
                    spawns,
                } = T2.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either8::T2(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7, T8),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either8::T3(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let Updated {
                    live_view: T3,
                    js_commands,
                    spawns,
                } = T3.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either8::T3(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7, T8),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either8::T4(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let Updated {
                    live_view: T4,
                    js_commands,
                    spawns,
                } = T4.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either8::T4(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7, T8),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either8::T5(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let Updated {
                    live_view: T5,
                    js_commands,
                    spawns,
                } = T5.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either8::T5(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7, T8),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either8::T6(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let Updated {
                    live_view: T6,
                    js_commands,
                    spawns,
                } = T6.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either8::T6(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7, T8),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either8::T7(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let Updated {
                    live_view: T7,
                    js_commands,
                    spawns,
                } = T7.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either8::T7(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7, T8),
                        render,
                    },
                    js_commands,
                    spawns,
                }
            }
            Either8::T8(msg) => {
                let Self {
                    views: (T1, T2, T3, T4, T5, T6, T7, T8),
                    render,
                } = self;
                let Updated {
                    live_view: T8,
                    js_commands,
                    spawns,
                } = T8.update(msg, data);
                let spawns = spawns
                    .into_iter()
                    .map(|future| Box::pin(async move { Either8::T8(future.await) }) as _)
                    .collect::<Vec<_>>();
                Updated {
                    live_view: Self {
                        views: (T1, T2, T3, T4, T5, T6, T7, T8),
                        render,
                    },
                    js_commands,
                    spawns,
                }
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
