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

//! Type config for watcher implementation to define the key, value, response and error types.

use std::error::Error;
use std::fmt::Debug;
use std::fmt::Display;
use std::future::Future;
use std::io;

/// A key-value change event: (key, old_value, new_value)
///
/// If the old_value is None, it means the key is newly created.
/// If the new_value is None, it means the key is deleted.
pub type KVChange<C> = (KeyOf<C>, Option<ValueOf<C>>, Option<ValueOf<C>>);

/// The type of keys in the watch system.
pub type KeyOf<C> = <C as TypeConfig>::Key;

/// The type of values associated with keys.
pub type ValueOf<C> = <C as TypeConfig>::Value;

/// The type of responses returned to watchers.
pub type ResponseOf<C> = <C as TypeConfig>::Response;

/// The type of errors returned to watchers.
pub type ErrorOf<C> = <C as TypeConfig>::Error;

/// A type configuration trait that defines the core types used by the watcher system.
///
/// This trait serves as a central configuration point for the watcher implementation,
/// allowing customization of key-value types and response handling.
///
/// # Type Parameters
///
/// - `Key`: The type used for keys in the watch system. Must be comparable and cloneable.
/// - `Value`: The type used for values associated with keys.
/// - `Response`: The type returned to watchers when changes occur.
/// - `Error`: The error type returned to watchers when operations fail.
///
/// Implementations of this trait provide the necessary type definitions and
/// behavior for creating responses from changes and handling errors.
pub trait TypeConfig
where Self: Debug + Clone + Copy + Sized + 'static
{
    /// The type of keys that are watched.
    type Key: Debug + Display + Clone + Ord + Send + Sync + 'static;

    /// The type of values that are watched.
    type Value: Debug + Clone + Send + Sync + 'static;

    /// The type of responses returned to watchers.
    type Response: Send + 'static;

    /// The type of errors returned to watchers.
    type Error: Error + Send + 'static;

    /// Create a response instance from a key-value change.
    fn new_response(change: KVChange<Self>) -> Self::Response;

    /// Create an error when the data source returns io::Error
    fn data_error(error: io::Error) -> Self::Error;

    /// Update the watcher count metrics by incrementing or decrementing by the given value.
    ///
    /// # Arguments
    /// * `delta` - The change in watcher count (positive for increment, negative for decrement)
    fn update_watcher_metrics(delta: i64);

    /// Spawn a task in the provided runtime.
    ///
    /// This is used to spawn a [`Dispatcher`] task running in the background.
    ///
    /// [`Dispatcher`]: crate::dispatch::Dispatcher
    fn spawn<T>(fut: T)
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static;
}
