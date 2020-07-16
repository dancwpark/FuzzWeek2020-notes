use std::io;
use std::path::Path;
use std::time::{Instant, Duration};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::process::{Command, ExitStatus};
use std::collections::BTreeSet;
use std::os::unix::process::ExitStatusExt;

/// Number of iterations to run per thread before reporting stats to 
/// global stats structure
const BATCH_SIZE: usize = 1000;

#[derive(Default)]
struct Statistics {
    /// Number of fuzz cases performed
    fuzz_cases: AtomicUsize, 
    crashes: AtomicUsize,
}

struct Rng(u64);
impl Rng {
    /// Create new rng
    fn new() -> Self {
        // get seed from python
        // import random
        // hex(random.randint(0, 2**64 - 1))
        // Also XORing seed by current uptime of processor so each
        // thread's rng is unique
        Rng(0x2839839283234 ^ unsafe { std::arch::x86_64::_rdtsc() })
    }

    /// generate a random number
    #[inline]
    fn rand(&mut self) -> usize { // xorshift
        let val = self.0;
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 43;
        val as usize
    }
}

/// Save 'inp' to disk with a uniquefilename basd on 'thr-id' and run it
/// through 'objdump' once, returning the status code from 'objdump'
fn fuzz<P: AsRef<Path>>(filename: P, inp: &[u8]) -> io::Result<ExitStatus> {
    // Write out the input to a temporary file
    std::fs::write(filename.as_ref(), inp)?;

    // Run objdump against the file
    let runner = Command::new("./objdump").args(&[
        "-x", 
        filename.as_ref().to_str().unwrap(),
    ]).output()?;

    Ok(runner.status)
}

/// A fuzz worker which fuzzes forever in a loop
fn worker(thr_id: usize, statistics: Arc<Statistics>,
          corpus: Arc<Vec<Vec<u8>>>) -> io::Result<()> {
    // create rng
    let mut rng = Rng::new();

    let filename = format!("tmpinput{}", thr_id);

    // input for fuzz case
    let mut fuzz_input = Vec::new();

    loop {
        for _ in 0..BATCH_SIZE {
            //Pick random entry from corpus
            let sel = rng.rand() % corpus.len();
            
            // copy random input from corpus into fuzz input
            fuzz_input.clear();
            fuzz_input.extend_from_slice(&corpus[sel]);

            // Randomly corrupt the input
            for _ in 0..(rng.rand() % 8) + 1 {
                let sel = rng.rand() % fuzz_input.len();
                fuzz_input[sel] = rng.rand() as u8;
            }

            let exit = fuzz(&filename, &fuzz_input)?;
            if let Some(11) = exit.signal() {
                // SIGSEV
                statistics.crashes.fetch_add(1, Ordering::SeqCst);
            }
        } 
        // Update statistics --> limits thrashing
        statistics.fuzz_cases.fetch_add(BATCH_SIZE, Ordering::SeqCst);
    }
}

fn main() -> io::Result<()> {
    // Load the inital corpus
    let mut corpus = BTreeSet::new();
    for filename in std::fs::read_dir("corpus")? {
        let filename = filename?.path();
        corpus.insert(std::fs::read(filename)?);
    }
    let corpus: Arc<Vec<Vec<u8>>> = Arc::new(corpus.into_iter().collect());

    print!("Loaded {} files into corpus\n", corpus.len());

    // Stats during fuzzing
    let stats = Arc::new(Statistics::default());

    for thr_id in 0..1 {
        // Spawn the thread
        let stats = stats.clone();
        let corpus = corpus.clone();
        std::thread::spawn(move || worker (thr_id, stats, corpus));
        // move copies the thr_id instead of pass by ref
        // without move, this caues a race condition
    }
    

    // Start a timer
    let start = Instant::now();

    loop {
        std::thread::sleep(Duration::from_millis(1000));
        
        let elapsed = start.elapsed().as_secs_f64();
        let cases = stats.fuzz_cases.load(Ordering::SeqCst);
        let crashes = stats.crashes.load(Ordering::SeqCst);
        print!("[{:10.6}] cases {:10} | fcps {:10.2} | crashes {:10}\n", 
               elapsed, cases, cases as f64 / elapsed, crashes);
    }

    // Unreachable because loop goes forever. Add wait for
    //  threads ending later
    // print!("{:?}\n", fuzz("asdf", b"asdf")?);

}








