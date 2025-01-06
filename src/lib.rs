use std::{
    fmt::Display,
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

#[derive(Debug)]
pub enum PoolCreationError {
    ZeroThreadsToCreate,
}
impl Display for PoolCreationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroThreadsToCreate => {
                write!(
                    formatter,
                    "The size should be grater than ZERO, at least ONE thread is required."
                )
            }
        }
    }
}
type Job = Box<dyn FnOnce() + Send + 'static>;
type Receiver = Arc<Mutex<mpsc::Receiver<Job>>>;

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

fn build_thread(retry_count: u8, receiver: Receiver, thread_id: usize) -> JoinHandle<()> {
    let thread_builder = thread::Builder::new();
    let receiver_clone = Arc::clone(&receiver);
    match thread_builder.spawn(move || loop {
        let message = receiver_clone
            .lock()
            .expect("Failed while locking a receiver.")
            .recv();

        match message {
            Ok(job) => {
                println!("Worker {thread_id} got a job; executing.");
                job();
            }
            Err(_) => {
                // At this point probably sender was dropped
                println!("Worker {thread_id} disconnected; shutting down.");
                break;
            }
        }
    }) {
        Ok(handle) => handle,
        Err(_) => {
            if retry_count == 5 {
                panic!("Couldn't create a new thread.")
            }
            eprintln!("Retries to create a new thread.");
            build_thread(retry_count + 1, receiver, thread_id)
        }
    }
}

impl Worker {
    pub fn new(id: usize, receiver: Receiver) -> Worker {
        Worker {
            id,
            thread: Option::Some(build_thread(0, receiver, id)),
        }
    }
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    /// Create a new ThreadPool
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # returns PoolCreationError
    ///
    /// The `build` function will return `PoolCreationError` if the size is zero.
    pub fn build(size: usize) -> Result<ThreadPool, PoolCreationError> {
        match size {
            0 => Err(PoolCreationError::ZeroThreadsToCreate),
            _ => {
                let mut workers = Vec::with_capacity(size);
                let (sender, receiver) = mpsc::channel::<Job>();
                let receiver = Arc::new(Mutex::new(receiver));

                for id in 0..size {
                    workers.push(Worker::new(id, Arc::clone(&receiver)));
                }
                Ok(ThreadPool {
                    workers,
                    sender: Option::Some(sender),
                })
            }
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        if let Some(sender) = &self.sender {
            sender.send(job).expect("Failed to send to a job.");
        };
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);
            if let Some(thread) = worker.thread.take() {
                thread
                    .join()
                    .expect(format!("Failed to join thread {}", worker.id).as_str());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation_error() {
        let pool = ThreadPool::build(0);
        assert_eq!(pool.is_err(), true);
    }

    #[test]
    fn test_pool_creation() {
        let pool = ThreadPool::build(4);
        assert_eq!(pool.is_ok(), true);
    }

    #[test]
    fn test_pool_execute() {
        // given
        let pool = ThreadPool::build(1).unwrap();
        let some_value = Arc::new(Mutex::new(0));
        let some_value_clone = Arc::clone(&some_value);
        let is_done = Arc::new(Mutex::new(false));
        let is_done_clone = Arc::clone(&is_done);

        // when
        pool.execute(move || {
            println!("Executing a job.");
            let mut locked_value = some_value.lock().unwrap();
            *locked_value = 1;
            *is_done.lock().unwrap() = true;
        });
        loop {
            match is_done_clone.try_lock() {
                Ok(is_done) => {
                    if *is_done {
                        break;
                    }
                }
                Err(_) => {}
            }
        }

        // then
        assert_eq!(*some_value_clone.lock().unwrap(), 1);
    }
}
