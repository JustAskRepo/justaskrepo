use std::any::{Any, TypeId, type_name};
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast, broadcast::error::RecvError};
use tokio::task::JoinHandle;
use tracing::{Instrument, debug, error, info_span};

use crate::shared_kernel::{domain_events::DomainEvent, error::AppError};

#[derive(Clone)]
pub struct EventBus {
    capacity: usize,
    channels: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Announce that something happened.
    ///
    /// Returns nothing, deliberately. Publishing to zero listeners is the
    /// normal state of a young system — `auth` will publish
    /// `UserAuthenticatedEvent` for weeks before `installations` exists to hear
    /// it — and every caller would have to decide what to do about "nobody is
    /// listening" when the honest answer is nothing. A login must never fail
    /// because a downstream module has not been written yet.
    ///
    /// `async` although it awaits only a lock today: the day this becomes NATS,
    /// publishing is a network call and every caller already has the `.await`.
    pub async fn publish<E: DomainEvent>(&self, event: E) {
        let name = event.event_name();

        let Some(sender) = self.sender_for::<E>().await else {
            debug!(event = name, "event published with no listeners");
            return;
        };

        match sender.send(event) {
            Ok(listeners) => debug!(event = name, listeners, "event published"),
            Err(_) => debug!(event = name, "event published with no listeners"),
        }
    }

    /// Run `handler` for every `E` published from here on, in its own task.
    ///
    /// The handle belongs to the calling module's `subscribe`, and from there to
    /// `main.rs`, so shutdown can wait for listeners to finish rather than
    /// cutting them off mid-handler.
    ///
    /// Events published before this call are not replayed — the bus carries
    /// notices, it does not keep a log.
    pub async fn subscribe<E, F, Fut>(&self, handler: F) -> JoinHandle<()>
    where
        E: DomainEvent,
        F: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        tokio::spawn(listen(self.receiver_for::<E>().await, handler))
    }

    /// `None` when nothing ever subscribed to `E`. Nothing is created on the
    /// publish path: a channel with no receivers would drop the event anyway,
    /// and this keeps publishing to a read lock.
    async fn sender_for<E: DomainEvent>(&self) -> Option<broadcast::Sender<E>> {
        self.channels
            .read()
            .await
            .get(&TypeId::of::<E>())
            .and_then(|slot| slot.downcast_ref::<broadcast::Sender<E>>())
            .cloned()
    }

    async fn receiver_for<E: DomainEvent>(&self) -> broadcast::Receiver<E> {
        let mut channels = self.channels.write().await;

        if let Some(sender) = channels
            .get(&TypeId::of::<E>())
            .and_then(|slot| slot.downcast_ref::<broadcast::Sender<E>>())
        {
            return sender.subscribe();
        }

        let (sender, receiver) = broadcast::channel::<E>(self.capacity);
        channels.insert(TypeId::of::<E>(), Box::new(sender));
        receiver
    }
}

/// The listener loop, written once so five modules do not each grow their own
/// copy of the lag handling, the error logging, and the shutdown behaviour.
async fn listen<E, F, Fut>(mut receiver: broadcast::Receiver<E>, handler: F)
where
    E: DomainEvent,
    F: Fn(E) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), AppError>> + Send + 'static,
{
    loop {
        match receiver.recv().await {
            Ok(event) => dispatch(&handler, event).await,

            // Never stop on lag: a subscription that quits on it stays dead.
            Err(RecvError::Lagged(missed)) => {
                error!(
                    event = type_name::<E>(),
                    missed, "event listener fell behind and lost events"
                );
            }

            Err(RecvError::Closed) => {
                debug!(
                    event = type_name::<E>(),
                    "event bus closed, listener stopping"
                );
                return;
            }
        }
    }
}

/// Runs one handler invocation and survives whatever it does.
async fn dispatch<E, F, Fut>(handler: &F, event: E)
where
    E: DomainEvent,
    F: Fn(E) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), AppError>> + Send + 'static,
{
    let name = event.event_name();
    let span = info_span!(
        "event_handler",
        event = name,
        correlation_id = %event.correlation_id()
    );

    // Own task, so a panic ends the handler and not the listener.
    match tokio::spawn(handler(event).instrument(span)).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => error!(event = name, %error, "event handler failed"),
        Err(join_error) if join_error.is_panic() => {
            error!(event = name, "event handler panicked, listener continues");
        }
        Err(_) => debug!(event = name, "event handler cancelled"),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::sync::{Notify, mpsc};
    use tokio::time::timeout;

    use super::*;
    use crate::shared_kernel::types::CorrelationId;

    #[derive(Debug, Clone)]
    struct ProbeHappenedEvent {
        id: u32,
        correlation_id: CorrelationId,
    }

    impl DomainEvent for ProbeHappenedEvent {
        fn event_name(&self) -> &'static str {
            "test.probe_happened"
        }

        fn correlation_id(&self) -> CorrelationId {
            self.correlation_id
        }
    }

    fn probe(id: u32) -> ProbeHappenedEvent {
        ProbeHappenedEvent {
            id,
            correlation_id: CorrelationId::new(),
        }
    }

    const PATIENCE: Duration = Duration::from_secs(5);

    /// Listeners run in their own tasks, so a delivery is never finished when
    /// `publish` returns. Every assertion waits, and fails rather than hangs.
    async fn next(delivered: &mut mpsc::UnboundedReceiver<u32>) -> u32 {
        match timeout(PATIENCE, delivered.recv()).await {
            Ok(Some(id)) => id,
            Ok(None) => panic!("the listener dropped its sender"),
            Err(_) => panic!("nothing delivered within {PATIENCE:?}"),
        }
    }

    /// A handler that reports every event it saw and always succeeds.
    fn reporter(to: mpsc::UnboundedSender<u32>) -> impl Fn(ProbeHappenedEvent) -> BoxedResult {
        move |event| {
            let to = to.clone();
            Box::pin(async move {
                let _ = to.send(event.id);
                Ok(())
            })
        }
    }

    type BoxedResult = std::pin::Pin<Box<dyn Future<Output = Result<(), AppError>> + Send>>;

    #[tokio::test]
    async fn a_published_event_reaches_its_listener() {
        let bus = EventBus::new(8);
        let (sender, mut delivered) = mpsc::unbounded_channel();

        let _listener = bus.subscribe(reporter(sender)).await;
        bus.publish(probe(7)).await;

        assert_eq!(next(&mut delivered).await, 7);
    }

    /// The whole point of a broadcast: adding a second listener later must not
    /// take the event away from the first.
    #[tokio::test]
    async fn every_listener_gets_its_own_copy() {
        let bus = EventBus::new(8);
        let (first_sender, mut first) = mpsc::unbounded_channel();
        let (second_sender, mut second) = mpsc::unbounded_channel();

        let _first = bus.subscribe(reporter(first_sender)).await;
        let _second = bus.subscribe(reporter(second_sender)).await;

        bus.publish(probe(3)).await;

        assert_eq!(next(&mut first).await, 3);
        assert_eq!(next(&mut second).await, 3);
    }

    /// The normal state of a young system, and explicitly not an error.
    #[tokio::test]
    async fn publishing_to_nobody_is_not_an_error() {
        let bus = EventBus::new(8);
        bus.publish(probe(1)).await;

        let (sender, mut delivered) = mpsc::unbounded_channel();
        let _listener = bus.subscribe(reporter(sender)).await;
        bus.publish(probe(2)).await;

        // 2, not 1: a subscription starts listening, it does not catch up.
        assert_eq!(next(&mut delivered).await, 2);
    }

    #[tokio::test]
    async fn a_listener_that_falls_behind_reports_the_miss_and_keeps_going() {
        const CAPACITY: usize = 2;
        const PUBLISHED: u32 = 10;

        let bus = EventBus::new(CAPACITY);
        let (sender, mut delivered) = mpsc::unbounded_channel();
        let gate = Arc::new(Notify::new());
        let held = Arc::clone(&gate);

        let _listener = bus
            .subscribe(move |event: ProbeHappenedEvent| {
                let sender = sender.clone();
                let held = Arc::clone(&held);
                async move {
                    let _ = sender.send(event.id);
                    // Park so the publisher can overrun the channel behind us.
                    if event.id == 0 {
                        held.notified().await;
                    }
                    Ok(())
                }
            })
            .await;

        bus.publish(probe(0)).await;
        assert_eq!(next(&mut delivered).await, 0, "the listener must be parked");

        for id in 1..=PUBLISHED {
            bus.publish(probe(id)).await;
        }
        // `notify_one` leaves a permit, so releasing cannot race the await.
        gate.notify_one();

        let mut seen = vec![];
        loop {
            let id = next(&mut delivered).await;
            seen.push(id);
            if id == PUBLISHED {
                break;
            }
        }

        // Reaching PUBLISHED proves it kept going; missing some proves it lagged.
        assert!(
            seen.len() < PUBLISHED as usize,
            "a capacity of {CAPACITY} should have dropped events, saw {seen:?}"
        );
    }

    /// The property `main.rs` shutdown leans on: the router drops the last
    /// `AppContext`, that drops the bus, and every listener then ends on its
    /// own so the drain loop has something finite to wait for.
    #[tokio::test]
    async fn a_listener_stops_when_the_bus_goes_away() {
        let bus = EventBus::new(8);
        let (sender, _delivered) = mpsc::unbounded_channel();
        let listener = bus.subscribe(reporter(sender)).await;

        drop(bus);

        match timeout(PATIENCE, listener).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => panic!("the listener ended abnormally: {error}"),
            Err(_) => panic!("still running {PATIENCE:?} after the bus was dropped"),
        }
    }

    #[tokio::test]
    async fn a_failing_handler_does_not_end_the_subscription() {
        let bus = EventBus::new(8);
        let (sender, mut delivered) = mpsc::unbounded_channel();

        let _listener = bus
            .subscribe(move |event: ProbeHappenedEvent| {
                let sender = sender.clone();
                async move {
                    let _ = sender.send(event.id);
                    if event.id == 1 {
                        return Err(AppError::Validation("deliberate".to_owned()));
                    }
                    Ok(())
                }
            })
            .await;

        bus.publish(probe(1)).await;
        bus.publish(probe(2)).await;

        assert_eq!(next(&mut delivered).await, 1);
        assert_eq!(next(&mut delivered).await, 2);
    }

    /// One bad event must not end a subscription. The alternative is a system
    /// that looks healthy and has silently stopped reacting.
    #[tokio::test]
    async fn a_panicking_handler_does_not_end_the_subscription() {
        let bus = EventBus::new(8);
        let (sender, mut delivered) = mpsc::unbounded_channel();

        let _listener = bus
            .subscribe(move |event: ProbeHappenedEvent| {
                let sender = sender.clone();
                async move {
                    let _ = sender.send(event.id);
                    assert!(event.id != 1, "deliberate panic inside a handler");
                    Ok(())
                }
            })
            .await;

        bus.publish(probe(1)).await;
        bus.publish(probe(2)).await;

        assert_eq!(next(&mut delivered).await, 1);
        assert_eq!(next(&mut delivered).await, 2);
    }
}
