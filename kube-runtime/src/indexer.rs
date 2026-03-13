use async_stream::stream;
use futures::{Stream, StreamExt};
use std::{hash::Hash, sync::Arc};

use crate::{reflector::Lookup, watcher};

pub trait Index<K> {
    fn apply(&self, obj: K);
    fn delete(&self, obj: &K);
    fn rehydrate(&self, objs: Vec<K>);
}

pub fn indexer<K, I: Index<K>, W>(index: Arc<I>, stream: W) -> impl Stream<Item = W::Item>
where
    K: Lookup + Clone,
    K::DynamicType: Eq + Hash + Clone,
    W: Stream<Item = watcher::Result<watcher::Event<K>>>,
{
    let mut stream = Box::pin(stream);
    stream! {
        let mut buffer = Vec::new();
        while let Some(event) = stream.next().await {
            match event {
                Ok(ev) => {
                    match &ev {
                        watcher::Event::Apply(obj) => {
                            index.apply(obj.clone());
                        }
                        watcher::Event::Delete(obj) => {
                            index.delete(&obj);
                        }
                        watcher::Event::Init => {
                            buffer = Vec::new();
                        }
                        watcher::Event::InitApply(obj) => {
                            buffer.push(obj.clone());
                        }
                        watcher::Event::InitDone => {
                            index.rehydrate(buffer);
                            buffer = Vec::new();
                        }
                    };

                    yield Ok(ev);
                },
                Err(ev) => yield Err(ev)
            }
        }
    }
}
