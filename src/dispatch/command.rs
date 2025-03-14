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

use futures::future::BoxFuture;

use crate::dispatch::Dispatcher;
use crate::type_config::KVChange;
use crate::type_config::TypeConfig;

/// A command sent to [`Dispatcher`].
///
/// [`Dispatcher`] is a single task that processes all commands serially in the order they are received.
///
/// Commands can be categorized into two types:
/// - Synchronous commands: These commands (like `Change` and `Func`) execute immediately and block
///   the dispatcher's thread until completion.
/// - Asynchronous commands: These commands (like `AsyncFunc` and `Future`) return a future that
///   will be awaited on the dispatcher's thread, allowing other tasks to make progress while waiting.
///
/// This command-based architecture ensures thread safety by serializing all operations on the dispatcher.
#[allow(clippy::type_complexity)]
pub enum Command<C>
where C: TypeConfig
{
    /// Submit a key-value change event to dispatcher.
    Change(KVChange<C>),

    /// Execute a function to the [`Dispatcher`].
    ///
    /// The function will be called with a mutable reference to the dispatcher,
    /// allowing it to modify the dispatcher's state or perform operations.
    /// This is a synchronous operation that blocks until completion.
    Func {
        req: Box<dyn FnOnce(&mut Dispatcher<C>) + Send + 'static>,
    },

    /// Send a function to [`Dispatcher`] to run it asynchronously.
    ///
    /// The function will be called with a mutable reference to the dispatcher,
    /// allowing it to modify the dispatcher's state or perform operations.
    /// The function returns a future that will be awaited on the dispatcher's thread.
    AsyncFunc {
        req: Box<dyn FnOnce(&mut Dispatcher<C>) -> BoxFuture<'static, ()> + Send + 'static>,
    },

    /// Send a future to [`Dispatcher`] to run it.
    ///
    /// The future will be awaited on the dispatcher's thread.
    Future(BoxFuture<'static, ()>),
}
