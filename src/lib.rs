use std::{
    fmt::Display,
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

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
    thread: thread::JoinHandle<()>,
}

fn build_thread(retry_count: u8, receiver: Receiver, thread_id: usize) -> JoinHandle<()> {
    let thread_builder = thread::Builder::new();
    let receiver_clone = Arc::clone(&receiver);
    match thread_builder.spawn(move || loop {
        let job = receiver_clone
            .lock()
            .expect("Failed while locking a receiver.")
            .recv()
            .expect("Couldn't receive a job.");

        println!("Worker {thread_id} got a job; executing.");

        job();
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
            thread: build_thread(0, receiver, id),
        }
    }
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
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
                Ok(ThreadPool { workers, sender })
            }
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.send(job).unwrap();
    }
}
