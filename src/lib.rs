use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self},
    },
    thread,
};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job: Job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let message = receiver.lock().unwrap().recv();

                match message {
                    Ok(job) => {
                        println!("Worker {id} got a job; executing.");

                        job();
                    }
                    Err(_) => {
                        println!("Worker {id} disconnected; shutting down.");
                        break;
                    }
                }
            }
        });
        Worker {
            id: id,
            thread: Some(thread),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn creates_thread_pool_with_given_size() {
        let pool = ThreadPool::new(4);
        assert_eq!(pool.workers.len(), 4);
    }

    #[test]
    fn executes_single_job() {
        let pool = ThreadPool::new(1);
        let flag = Arc::new(Mutex::new(false));
        let flag_clone = Arc::clone(&flag);

        pool.execute(move || {
            let mut val = flag_clone.lock().unwrap();
            *val = true;
        });

        // Give the thread some time to run the job
        thread::sleep(Duration::from_millis(100));

        assert_eq!(*flag.lock().unwrap(), true);
    }

    #[test]
    fn executes_multiple_jobs_across_workers() {
        let pool = ThreadPool::new(4);
        let counter = Arc::new(Mutex::new(0));

        for _ in 0..8 {
            let counter_clone = Arc::clone(&counter);
            pool.execute(move || {
                let mut num = counter_clone.lock().unwrap();
                *num += 1;
            });
        }

        thread::sleep(Duration::from_millis(200));

        assert_eq!(*counter.lock().unwrap(), 8);
    }

    #[test]
    fn thread_pool_drops_gracefully() {
        let pool = ThreadPool::new(2);
        let result = Arc::new(Mutex::new(0));

        for _ in 0..5 {
            let result_clone = Arc::clone(&result);
            pool.execute(move || {
                thread::sleep(Duration::from_millis(50));
                let mut val = result_clone.lock().unwrap();
                *val += 1;
            });
        }

        // Drop happens here when pool goes out of scope
        drop(pool);

        assert_eq!(*result.lock().unwrap(), 5);
    }

    #[test]
    fn thread_pool_executes_jobs_concurrently() {
        let pool = ThreadPool::new(2);
        let start = Instant::now();

        for _ in 0..2 {
            pool.execute(|| {
                thread::sleep(Duration::from_millis(100));
            });
        }

        thread::sleep(Duration::from_millis(150));
        let elapsed = start.elapsed();

        // If run sequentially, would take ~200ms, but with 2 threads should be less
        assert!(elapsed < Duration::from_millis(200));
    }
}
