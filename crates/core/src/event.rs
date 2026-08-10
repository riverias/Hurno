/// Minimal event bus — enqueue and drain each frame
#[derive(Default)]
pub struct EventBus<E> {
    queue: Vec<E>,
}

impl<E> EventBus<E> {
    pub fn push(&mut self, e: E) { self.queue.push(e); }
    pub fn drain(&mut self) -> impl Iterator<Item = E> + '_ {
        self.queue.drain(..)
    }
}
