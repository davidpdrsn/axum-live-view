/// Spawn a future that is required to yield `()`.
///
/// This means the future is required to handle all errors.
pub(crate) fn spawn_unit<F>(future: F) -> tokio::task::JoinHandle<()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    tokio::spawn(future)
}
