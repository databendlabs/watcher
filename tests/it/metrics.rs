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

use std::collections::Bound;
use std::future::Future;
use std::io;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering;

use tokio::sync::mpsc;
use watcher::dispatch::Dispatcher;
use watcher::type_config::KVChange;
use watcher::type_config::KeyOf;
use watcher::type_config::TypeConfig;
use watcher::type_config::ValueOf;
use watcher::EventFilter;
use watcher::KeyRange;

// Sender count metrics container.
static SENDER_COUNT: AtomicI64 = AtomicI64::new(0);

// Only Debug is actually needed for the test framework
#[derive(Debug, Copy, Clone)]
struct Types {}

impl TypeConfig for Types {
    type Key = String;
    type Value = String;
    type Response = (String, Option<String>, Option<String>);
    type Error = io::Error;

    fn new_initialize_response(key: KeyOf<Self>, value: ValueOf<Self>) -> Self::Response {
        (key, None, Some(value))
    }

    fn new_change_response(change: KVChange<Self>) -> Self::Response {
        change
    }

    fn data_error(error: io::Error) -> Self::Error {
        error
    }

    fn update_watcher_metrics(delta: i64) {
        SENDER_COUNT.fetch_add(delta, Ordering::Relaxed);
    }

    #[allow(clippy::disallowed_methods)]
    fn spawn<T>(fut: T)
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        tokio::spawn(fut);
    }
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn test_metrics() {
    let handle = Dispatcher::<Types>::spawn();

    {
        // Add a watcher, sender count should be 1

        let (tx, _rx) = mpsc::channel(10);
        let weak_sender = handle
            .add_watcher(rng("a", "c"), EventFilter::update(), tx)
            .await
            .unwrap();

        let sender_count = SENDER_COUNT.load(Ordering::Relaxed);
        assert_eq!(sender_count, 1);

        // Remove a watcher, sender count should still be 1, because the watcher is not dropped yet

        let sender = weak_sender.upgrade().unwrap();

        handle.remove_watcher(sender.clone()).await.unwrap();
        let sender_count = SENDER_COUNT.load(Ordering::Relaxed);
        assert_eq!(sender_count, 1);

        // Remove Again, sender count should be 1, because the watcher is not dropped yet
        handle.remove_watcher(sender.clone()).await.unwrap();
        let sender_count = SENDER_COUNT.load(Ordering::Relaxed);
        assert_eq!(sender_count, 1);
    }

    // Drop the handle, sender count should be 0

    let sender_count = SENDER_COUNT.load(Ordering::Relaxed);
    assert_eq!(sender_count, 0);
}

fn s(x: &str) -> String {
    x.to_string()
}

fn rng(start: &str, end: &str) -> KeyRange<Types> {
    (Bound::Included(s(start)), Bound::Excluded(s(end)))
}
