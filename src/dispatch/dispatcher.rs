// Copyright 2021 Datafuse Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::BTreeSet;
use std::future::Future;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Weak;

use log::debug;
use log::info;
use log::warn;
use span_map::SpanMap;
use tokio::sync::mpsc;

use crate::dispatch::command::Command;
use crate::dispatch::dispatcher_handle::DispatcherHandle;
use crate::event_filter::EventFilter;
use crate::type_config::KVChange;
use crate::type_config::TypeConfig;
use crate::watch_stream::WatchStreamSender;
use crate::KeyRange;
use crate::WatchDesc;
use crate::WatchResult;
use crate::WatcherId;

/// Receives events from event sources via `rx` and dispatches them to interested watchers.
///
/// The [`Dispatcher`] acts as a central hub for the watch system, managing
/// subscriptions and ensuring that each watcher receives only the events
/// they have registered interest in. It maintains a mapping of watchers
/// and their watch descriptors to efficiently route events.
pub struct Dispatcher<C>
where C: TypeConfig
{
    rx: mpsc::UnboundedReceiver<Command<C>>,

    watchers: SpanMap<C::Key, Arc<WatchStreamSender<C>>>,
}

impl<C> Drop for Dispatcher<C>
where C: TypeConfig
{
    fn drop(&mut self) {
        debug!("watch-event-Dispatcher is dropped");
    }
}

impl<C> Dispatcher<C>
where C: TypeConfig
{
    /// Create a new dispatcher and its handle.
    ///
    /// Returns a tuple of the handle and the dispatcher's main loop future.
    /// The caller is responsible for spawning the future on their chosen runtime.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (handle, fut) = Dispatcher::<MyTypes>::create();
    /// tokio::spawn(fut);
    /// ```
    pub fn create() -> (
        DispatcherHandle<C>,
        impl Future<Output = ()> + Send + 'static,
    ) {
        let (tx, rx) = mpsc::unbounded_channel();

        let dispatcher = Dispatcher {
            rx,
            watchers: SpanMap::new(),
        };

        (DispatcherHandle::new(tx), dispatcher.main())
    }

    async fn main(mut self) {
        while let Some(event) = self.rx.recv().await {
            match event {
                Command::Change(kv_change) => {
                    self.dispatch(kv_change).await;
                }
                Command::Func { req } => req(&mut self),
                Command::AsyncFunc { req } => req(&mut self).await,
                Command::Future(fu) => fu.await,
            }
        }

        info!("watch-event-Dispatcher: all event senders are closed(dropped). quit");
    }

    /// Dispatch a kv change event to interested watchers.
    async fn dispatch(&mut self, kv_change: KVChange<C>) {
        let is_delete = kv_change.2.is_none();
        let event_type = if is_delete {
            EventFilter::DELETE
        } else {
            EventFilter::UPDATE
        };

        let mut removed = vec![];

        debug!("watch-event-Dispatcher: dispatch event {:?}", kv_change);

        for sender in self.watchers.get(&kv_change.0) {
            debug!(
                "watch-event-Dispatcher: dispatch event to watcher {}, kv_change: {:?}",
                sender, kv_change
            );
            let interested = sender.desc.interested;

            if !interested.accepts_event_type(event_type) {
                continue;
            }

            let resp = C::new_change_response(kv_change.clone());
            if let Err(_err) = sender.send(resp).await {
                warn!(
                    "watch-event-Dispatcher: fail to send to watcher {:?}; close this stream",
                    sender
                );
                removed.push(sender.clone());
            };
        }

        for sender in removed {
            self.remove_watcher(sender);
        }
    }

    /// Create and insert a new watch stream sender with the given key range and filter.
    pub fn add_watcher(
        &mut self,
        rng: KeyRange<C>,
        filter: EventFilter,
        tx: mpsc::Sender<WatchResult<C>>,
    ) -> Weak<WatchStreamSender<C>> {
        info!(
            "watch-event-Dispatcher::add_watcher: range: {:?}, filter: {}",
            rng, filter
        );

        let stream_sender = Self::new_watch_stream_sender(rng, filter, tx);

        self.watchers
            .insert(stream_sender.desc.key_range.clone(), stream_sender.clone());

        Arc::downgrade(&stream_sender)
    }

    /// Create a new watch stream sender with the given key range and filter.
    ///
    /// It does not add the sender to the dispatcher, use `insert_stream_sender` to insert.
    ///
    /// Or use `add_watcher` as a combined function to create and insert the sender.
    pub fn new_watch_stream_sender(
        rng: KeyRange<C>,
        filter: EventFilter,
        tx: mpsc::Sender<WatchResult<C>>,
    ) -> Arc<WatchStreamSender<C>> {
        info!(
            "watch-event-Dispatcher::new_stream_sender: range: {:?}, filter: {}",
            rng, filter
        );

        let desc = Self::new_watch_desc(rng, filter);

        Arc::new(WatchStreamSender::new(desc, tx))
    }

    /// Insert an existing watch stream sender into the dispatcher.
    pub fn insert_watch_stream_sender(&mut self, stream_sender: Arc<WatchStreamSender<C>>) {
        self.watchers
            .insert(stream_sender.desc.key_range.clone(), stream_sender.clone());
    }

    fn new_watch_desc(key_range: KeyRange<C>, interested: EventFilter) -> WatchDesc<C> {
        let watcher_id = Self::next_watcher_id();
        WatchDesc::new(watcher_id, interested, key_range)
    }

    fn next_watcher_id() -> WatcherId {
        static NEXT_WATCHER_ID: AtomicU64 = AtomicU64::new(1u64);
        NEXT_WATCHER_ID.fetch_add(1, Ordering::Relaxed) as i64
    }

    pub fn remove_watcher(&mut self, stream_sender: Arc<WatchStreamSender<C>>) {
        info!(
            "watch-event-Dispatcher::remove_watcher: {:?}",
            stream_sender
        );

        self.watchers.remove(.., stream_sender);
    }

    #[allow(clippy::mutable_key_type)]
    pub fn watch_senders(&self) -> BTreeSet<&Arc<WatchStreamSender<C>>> {
        self.watchers.values(..)
    }
}
